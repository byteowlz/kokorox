//! Black-box protocol tests intentionally do not share the worker's codec.
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

struct Worker {
    child: Child,
    input: ChildStdin,
    output: Receiver<(u8, u32, Vec<u8>)>,
}

impl Worker {
    fn spawn(model: &str, voices: &str, voice: &str, language: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kokorox-tts-worker"))
            .args([
                "--serve",
                "--model",
                model,
                "--voices",
                voices,
                "--voice",
                voice,
                "--language",
                language,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let (send, output) = mpsc::channel();
        std::thread::spawn(move || loop {
            let mut header = [0; 9];
            if stdout.read_exact(&mut header).is_err() {
                break;
            }
            let len = u32::from_le_bytes(header[5..9].try_into().unwrap()) as usize;
            // Catches text/log contamination rather than allocating arbitrary bytes.
            if len > 1024 * 1024 {
                break;
            }
            let mut payload = vec![0; len];
            if stdout.read_exact(&mut payload).is_err() {
                break;
            }
            if send
                .send((
                    header[0],
                    u32::from_le_bytes(header[1..5].try_into().unwrap()),
                    payload,
                ))
                .is_err()
            {
                break;
            }
        });
        Self {
            child,
            input,
            output,
        }
    }

    fn receive(&self) -> (u8, u32, Vec<u8>) {
        self.output
            .recv_timeout(Duration::from_secs(60))
            .expect("worker frame, with no stdout log contamination")
    }

    fn send(&mut self, kind: u8, id: u32, payload: &[u8]) {
        let mut bytes = vec![kind];
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        self.input.write_all(&bytes).unwrap();
        self.input.flush().unwrap();
    }

    fn wait(&mut self, success: bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.success(), success);
                return;
            }
            assert!(Instant::now() < deadline, "worker did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn invalid_assets_report_a_binary_error_without_ready_or_logs() {
    let missing =
        std::env::temp_dir().join(format!("kokorox-missing-model-{}", std::process::id()));
    let mut worker = Worker::spawn(
        missing.to_str().unwrap(),
        missing.to_str().unwrap(),
        "af_sarah",
        "en-us",
    );
    let (kind, id, payload) = worker.receive();
    assert_eq!((kind, id), (5, 0));
    assert!(String::from_utf8(payload)
        .unwrap()
        .contains("local asset does not exist"));
    worker.wait(false);
}

#[test]
#[ignore = "requires local KOKOROX_TEST_MODEL and KOKOROX_TEST_VOICES (no downloads)"]
fn real_worker_synthesis_and_optional_deterministic_pcm_parity() {
    let model = std::env::var("KOKOROX_TEST_MODEL").unwrap();
    let voices = std::env::var("KOKOROX_TEST_VOICES").unwrap();
    let voice = std::env::var("KOKOROX_TEST_VOICE").unwrap_or("af_sarah".into());
    let language = std::env::var("KOKOROX_TEST_LANGUAGE").unwrap_or("en-us".into());
    let default_corpus = std::env::var("KOKOROX_TEST_TEXT").is_err();
    // Cross the core's 500-token split boundary, not only the worker's PCM
    // block boundary. German/stochastic smoke runs override this corpus.
    let text = std::env::var("KOKOROX_TEST_TEXT")
        .unwrap_or_else(|_| "Hello from Kokoro. This is a second sentence. ".repeat(12));
    let mut worker = Worker::spawn(&model, &voices, &voice, &language);
    assert_eq!(worker.receive(), (1, 0, Vec::new()));
    worker.send(1, 41, text.as_bytes()); // raw text, never JSON
    let (kind, id, payload) = worker.receive();
    assert_eq!((kind, id), (2, 41));
    assert_eq!(payload, 24000u32.to_le_bytes());
    let mut pcm = Vec::new();
    loop {
        let (kind, id, payload) = worker.receive();
        assert_eq!(id, 41);
        match kind {
            3 => {
                assert_eq!(payload.len() % 2, 0);
                pcm.extend(payload);
            }
            4 => {
                assert!(payload.is_empty());
                break;
            }
            5 => panic!("inference error: {}", String::from_utf8_lossy(&payload)),
            _ => panic!("unexpected output frame {kind}"),
        }
    }
    assert!(pcm.len() > 4800, "at least 100 ms of audio");
    assert!(pcm.iter().any(|byte| *byte != 0), "non-silent speech");
    worker.send(3, 0, &[]);
    worker.wait(true);
    if let Ok(output) = std::env::var("KOKOROX_TEST_PCM_OUTPUT") {
        std::fs::write(PathBuf::from(output), &pcm).unwrap();
    }
    // Some fine-tune exports contain RandomNormalLike/RandomUniformLike
    // operators. Independent generations cannot have byte-identical audio.
    if std::env::var("KOKOROX_TEST_STOCHASTIC").as_deref() == Ok("1") {
        eprintln!("stochastic model: audio smoke test only, deterministic parity skipped");
        return;
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let engine = runtime.block_on(kokorox::tts::koko::TTSKoko::from_paths(&model, &voices));
    let buffered = engine
        .tts_raw_audio(&text, &language, &voice, 1.0, None, false, true, false)
        .unwrap();
    let mut streamed = Vec::new();
    let mut inference_chunks = 0;
    engine
        .tts_stream_audio(
            &text,
            &language,
            &voice,
            1.0,
            None,
            false,
            true,
            false,
            &mut |chunk| {
                streamed.extend_from_slice(chunk);
                inference_chunks += 1;
                true
            },
        )
        .unwrap();
    if default_corpus {
        assert!(inference_chunks > 1, "exercise multiple inference chunks");
    }
    assert_eq!(
        buffered, streamed,
        "buffered and streaming core must be identical"
    );
    let expected: Vec<u8> = buffered
        .iter()
        .flat_map(|sample| {
            let pcm = (sample.clamp(-1.0, 1.0) * 32768.0).clamp(-32768.0, 32767.0) as i16;
            pcm.to_le_bytes()
        })
        .collect();
    assert_eq!(
        pcm, expected,
        "worker must preserve inference samples (only PCM quantization)"
    );
}
