//! Model-independent worker lifecycle. Output and cancellation share one lock,
//! so no audio is written after the reader has observed a cancellation.
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::protocol::{
    self, Frame, AUDIO_CHUNK, AUDIO_DONE, AUDIO_START, CANCEL, ERROR, READY, SHUTDOWN, SPEAK,
};

const QUEUE_SIZE: usize = 8;

struct Session<W> {
    writer: W,
    requests: HashMap<u32, bool>, // accepted id -> canceled
    stopping: bool,
}

impl<W: Write> Session<W> {
    fn write(&mut self, kind: u8, id: u32, payload: &[u8]) -> io::Result<()> {
        protocol::write_frame(&mut self.writer, kind, id, payload)
    }

    fn active(&self, id: u32) -> bool {
        !self.stopping && self.requests.get(&id) == Some(&false)
    }

    fn accept(&mut self, frame: &Frame) -> io::Result<bool> {
        let message = if frame.id == 0 {
            Some("request id 0 is reserved")
        } else if self.requests.contains_key(&frame.id) {
            Some("request id is already pending")
        } else if std::str::from_utf8(&frame.payload).map_or(true, |text| text.trim().is_empty()) {
            Some("speak must contain non-empty UTF-8 text")
        } else {
            None
        };
        if let Some(message) = message {
            self.write(ERROR, frame.id, message.as_bytes())?;
            return Ok(false);
        }
        self.requests.insert(frame.id, false);
        Ok(true)
    }

    fn finish(&mut self, id: u32, result: Result<(), String>) -> io::Result<()> {
        // Cancellation is terminal audio_done, matching spqx; errors are
        // terminal error without a success completion.
        let canceled = self.requests.remove(&id).unwrap_or(true);
        if self.stopping {
            return Ok(());
        }
        match result {
            Err(message) if !canceled => self.write(ERROR, id, message.as_bytes()),
            _ => self.write(AUDIO_DONE, id, &[]),
        }
    }
}

/// Serve a bounded request queue. The independent input thread observes cancel
/// while inference is running. EOF drains accepted work; explicit shutdown
/// immediately fences all output and discards queued work.
pub fn serve<R, W, F>(
    reader: R,
    writer: W,
    sample_rate: u32,
    blocksize: usize,
    mut synthesize: F,
) -> io::Result<()>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
    F: FnMut(&str, &mut dyn FnMut(&[f32]) -> bool) -> Result<(), String>,
{
    if sample_rate == 0 || blocksize == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "zero sample rate or blocksize",
        ));
    }
    let shared = Arc::new(Mutex::new(Session {
        writer,
        requests: HashMap::new(),
        stopping: false,
    }));
    shared.lock().unwrap().write(READY, 0, &[])?;
    let (sender, receiver) = mpsc::sync_channel(QUEUE_SIZE);
    let input_session = Arc::clone(&shared);
    let input = thread::spawn(move || read_commands(reader, sender, input_session));

    let format = AudioFormat {
        sample_rate,
        blocksize,
    };
    for frame in receiver {
        if shared.lock().unwrap().stopping {
            break;
        }
        handle_speak(frame, &shared, &format, &mut synthesize)?;
    }
    input
        .join()
        .map_err(|_| io::Error::other("input thread panicked"))?
}

struct AudioFormat {
    sample_rate: u32,
    blocksize: usize,
}

fn handle_speak<W: Write, F>(
    frame: Frame,
    shared: &Arc<Mutex<Session<W>>>,
    format: &AudioFormat,
    synthesize: &mut F,
) -> io::Result<()>
where
    F: FnMut(&str, &mut dyn FnMut(&[f32]) -> bool) -> Result<(), String>,
{
    {
        let mut session = shared.lock().unwrap();
        if !session.active(frame.id) {
            return session.finish(frame.id, Ok(()));
        }
        session.write(AUDIO_START, frame.id, &format.sample_rate.to_le_bytes())?;
    }
    let mut output_error = None;
    let result = synthesize(
        std::str::from_utf8(&frame.payload).unwrap().trim(),
        &mut |samples| match emit_samples(shared, frame.id, samples, format.blocksize) {
            Ok(active) => active,
            Err(error) => {
                output_error = Some(error);
                false
            }
        },
    );
    // Do not wait for a reader blocked on stdin after a broken output.
    if let Some(error) = output_error {
        return Err(error);
    }
    shared.lock().unwrap().finish(frame.id, result)
}

fn emit_samples<W: Write>(
    shared: &Arc<Mutex<Session<W>>>,
    id: u32,
    samples: &[f32],
    blocksize: usize,
) -> io::Result<bool> {
    for chunk in samples.chunks(blocksize) {
        // Release between blocks so a reader can observe cancel promptly.
        let mut session = shared.lock().unwrap();
        if !session.active(id) {
            return Ok(false);
        }
        session.write(AUDIO_CHUNK, id, &protocol::pcm_bytes(chunk))?;
    }
    Ok(shared.lock().unwrap().active(id))
}

fn read_commands<R: Read, W: Write>(
    mut reader: R,
    sender: mpsc::SyncSender<Frame>,
    shared: Arc<Mutex<Session<W>>>,
) -> io::Result<()> {
    loop {
        let frame = match Frame::read(&mut reader) {
            Ok(Some(frame)) => frame,
            Ok(None) => return Ok(()),
            Err(error) => {
                let mut session = shared.lock().unwrap();
                session.stopping = true;
                session.write(ERROR, 0, error.to_string().as_bytes())?;
                return Err(error);
            }
        };
        let mut session = shared.lock().unwrap();
        match frame.kind {
            CANCEL => {
                // Unknown/finished ids are ignored; never accumulate tombstones.
                if let Some(canceled) = session.requests.get_mut(&frame.id) {
                    *canceled = true;
                }
            }
            SHUTDOWN => {
                session.stopping = true;
                return Ok(());
            }
            SPEAK if session.accept(&frame)? => match sender.try_send(frame) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(frame)) => {
                    session.requests.remove(&frame.id);
                    session.write(ERROR, frame.id, b"worker queue is full")?;
                }
                Err(mpsc::TrySendError::Disconnected(_)) => return Ok(()),
            },
            SPEAK => {}
            _ => session.write(ERROR, frame.id, b"unknown frame type")?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn command(kind: u8, id: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        protocol::write_frame(&mut out, kind, id, payload).unwrap();
        out
    }

    #[derive(Default)]
    struct BufferedFrames {
        pending: Vec<u8>,
        flushed: Vec<u8>,
    }
    #[derive(Clone, Default)]
    struct Output(Arc<Mutex<BufferedFrames>>);
    impl Write for Output {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().pending.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            let mut frames = self.0.lock().unwrap();
            let pending = std::mem::take(&mut frames.pending);
            frames.flushed.extend_from_slice(&pending);
            Ok(())
        }
    }
    impl Output {
        fn frames(&self) -> Vec<Frame> {
            let bytes = self.0.lock().unwrap().flushed.clone();
            let mut reader = bytes.as_slice();
            let mut frames = Vec::new();
            while let Some(frame) = Frame::read(&mut reader).unwrap() {
                frames.push(frame);
            }
            frames
        }
    }

    #[test]
    fn raw_text_rate_and_pcm_match_spqx_without_json() {
        let input = command(SPEAK, 7, "Hallo Grüße".as_bytes());
        let output = Output::default();
        serve(
            Cursor::new(input),
            output.clone(),
            24000,
            2,
            |text, emit| {
                assert_eq!(text, "Hallo Grüße");
                assert!(emit(&[0.0, 0.5, -1.0]));
                Ok(())
            },
        )
        .unwrap();
        let frames = output.frames();
        assert_eq!(
            frames.iter().map(|f| f.kind).collect::<Vec<_>>(),
            [READY, AUDIO_START, AUDIO_CHUNK, AUDIO_CHUNK, AUDIO_DONE]
        );
        assert!(frames[0].payload.is_empty());
        assert_eq!(frames[1].payload, 24000u32.to_le_bytes());
        assert_eq!(frames[2].payload, [0, 0, 0, 64]);
        assert_eq!(frames[3].payload, [0, 128]);
    }

    #[test]
    fn invalid_requests_do_not_kill_worker_and_error_is_terminal() {
        let input = [
            command(SPEAK, 0, b"reserved"),
            command(SPEAK, 1, b" \n"),
            command(SPEAK, 2, &[0xff]),
            command(77, 3, b""),
            command(SPEAK, 4, b"valid"),
        ]
        .concat();
        let output = Output::default();
        serve(
            Cursor::new(input),
            output.clone(),
            24000,
            4096,
            |text, _| {
                assert_eq!(text, "valid");
                Err("inference failure".into())
            },
        )
        .unwrap();
        let frames = output.frames();
        for id in 0..=4 {
            assert!(frames.iter().any(|f| f.id == id && f.kind == ERROR));
        }
        assert!(!frames.iter().any(|f| f.kind == AUDIO_DONE));
    }

    // A live input, not a preloaded Cursor, exercises cancellation during
    // inference. The reader's unknown-frame error is an ordering barrier:
    // observing it proves the preceding cancel/shutdown was already consumed.
    struct Input {
        receiver: mpsc::Receiver<Vec<u8>>,
        current: Cursor<Vec<u8>>,
        finished: Option<mpsc::Sender<()>>,
    }
    impl Drop for Input {
        fn drop(&mut self) {
            if let Some(sender) = self.finished.take() {
                let _ = sender.send(());
            }
        }
    }
    impl Read for Input {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.current.position() as usize == self.current.get_ref().len() {
                self.current = match self.receiver.recv() {
                    Ok(bytes) => Cursor::new(bytes),
                    Err(_) => return Ok(0),
                };
            }
            self.current.read(buf)
        }
    }

    #[test]
    fn active_and_queued_cancel_are_fenced_and_next_request_works() {
        let output = Output::default();
        let observed = output.clone();
        let (send, receive) = mpsc::channel();
        let (started_send, started_receive) = mpsc::channel();
        let (resume_send, resume_receive) = mpsc::channel();
        let worker = thread::spawn(move || {
            serve(
                Input {
                    receiver: receive,
                    current: Cursor::new(Vec::new()),
                    finished: None,
                },
                output,
                24000,
                1,
                |text, emit| {
                    if text == "first" {
                        assert!(emit(&[0.5]));
                        started_send.send(()).unwrap();
                        resume_receive.recv().unwrap();
                        assert!(!emit(&[0.25]));
                    } else {
                        assert_eq!(text, "third"); // queued second must never synthesize
                        assert!(emit(&[-0.5]));
                    }
                    Ok(())
                },
            )
        });
        send.send(command(SPEAK, 1, b"first")).unwrap();
        started_receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        send.send(command(SPEAK, 2, b"second")).unwrap();
        send.send(command(CANCEL, 1, b"")).unwrap();
        send.send(command(CANCEL, 2, b"")).unwrap();
        send.send(command(77, 99, b"")).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !observed
            .frames()
            .iter()
            .any(|f| f.id == 99 && f.kind == ERROR)
        {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        send.send(command(SPEAK, 3, b"third")).unwrap();
        resume_send.send(()).unwrap();
        drop(send); // clean EOF drains accepted requests
        worker.join().unwrap().unwrap();
        let frames = observed.frames();
        assert_eq!(
            frames
                .iter()
                .filter(|f| f.id == 1 && f.kind == AUDIO_CHUNK)
                .count(),
            1
        );
        assert!(!frames.iter().any(|f| f.id == 2 && f.kind == AUDIO_START));
        assert!(frames.iter().any(|f| f.id == 2 && f.kind == AUDIO_DONE));
        assert!(frames.iter().any(|f| f.id == 3 && f.kind == AUDIO_CHUNK));
        assert!(frames.iter().any(|f| f.id == 3 && f.kind == AUDIO_DONE));
    }

    #[test]
    fn shutdown_fences_inference_and_discards_queued_work() {
        let output = Output::default();
        let observed = output.clone();
        let (send, receive) = mpsc::channel();
        let (started_send, started_receive) = mpsc::channel();
        let (resume_send, resume_receive) = mpsc::channel();
        let (finished_send, finished_receive) = mpsc::channel();
        let worker = thread::spawn(move || {
            serve(
                Input {
                    receiver: receive,
                    current: Cursor::new(Vec::new()),
                    finished: Some(finished_send),
                },
                output,
                24000,
                1,
                |text, emit| {
                    assert_eq!(text, "first");
                    started_send.send(()).unwrap();
                    resume_receive.recv().unwrap();
                    assert!(!emit(&[0.5]));
                    Ok(())
                },
            )
        });
        send.send(command(SPEAK, 1, b"first")).unwrap();
        started_receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        send.send(command(SPEAK, 2, b"queued")).unwrap();
        send.send(command(SHUTDOWN, 0, b"")).unwrap();
        // Input destruction acknowledges shutdown before inference resumes.
        finished_receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        resume_send.send(()).unwrap();
        worker.join().unwrap().unwrap();
        assert_eq!(
            observed.frames().iter().map(|f| f.kind).collect::<Vec<_>>(),
            [READY, AUDIO_START]
        );
    }

    #[test]
    fn queue_overflow_and_unknown_cancel_do_not_grow_request_state() {
        let output = Output::default();
        let shared = Arc::new(Mutex::new(Session {
            writer: output.clone(),
            requests: HashMap::new(),
            stopping: false,
        }));
        let (send, _receive) = mpsc::sync_channel(1);
        let input = [
            command(CANCEL, 99, b""),
            command(SPEAK, 1, b"accepted"),
            command(SPEAK, 2, b"overflow"),
            command(CANCEL, 77, b""),
        ]
        .concat();
        read_commands(Cursor::new(input), send, shared.clone()).unwrap();
        assert_eq!(shared.lock().unwrap().requests.len(), 1);
        assert_eq!(
            output
                .frames()
                .iter()
                .map(|f| (f.kind, f.id))
                .collect::<Vec<_>>(),
            [(ERROR, 2)]
        );
    }

    #[test]
    fn malformed_input_reports_error_and_shutdown_emits_no_audio() {
        let output = Output::default();
        assert!(serve(
            Cursor::new(vec![1, 2]),
            output.clone(),
            24000,
            4096,
            |_, _| panic!("not called")
        )
        .is_err());
        assert_eq!(output.frames().last().unwrap().kind, ERROR);
        let output = Output::default();
        serve(
            Cursor::new(command(SHUTDOWN, 0, b"")),
            output.clone(),
            24000,
            4096,
            |_, _| panic!("not called"),
        )
        .unwrap();
        assert_eq!(output.frames().len(), 1);
    }
}
