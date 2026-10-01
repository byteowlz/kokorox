//! Wire-compatible with spqx: raw-text speak, empty ready, rate in audio_start.
use std::io::{self, Read, Write};

pub const SPEAK: u8 = 1;
pub const CANCEL: u8 = 2;
pub const SHUTDOWN: u8 = 3;
pub const READY: u8 = 1;
pub const AUDIO_START: u8 = 2;
pub const AUDIO_CHUNK: u8 = 3;
pub const AUDIO_DONE: u8 = 4;
pub const ERROR: u8 = 5;
pub const MAX_PAYLOAD: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub id: u32,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn read(reader: &mut impl Read) -> io::Result<Option<Self>> {
        let mut header = [0; 9];
        // Only EOF exactly on a frame boundary is clean.
        loop {
            match reader.read(&mut header[..1]) {
                Ok(0) => return Ok(None),
                Ok(_) => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        reader.read_exact(&mut header[1..])?;
        let id = u32::from_le_bytes(header[1..5].try_into().unwrap());
        let len = u32::from_le_bytes(header[5..9].try_into().unwrap()) as usize;
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame exceeds 1 MiB",
            ));
        }
        let mut payload = vec![0; len];
        reader.read_exact(&mut payload)?;
        Ok(Some(Self {
            kind: header[0],
            id,
            payload,
        }))
    }
}

pub fn write_frame(writer: &mut impl Write, kind: u8, id: u32, payload: &[u8]) -> io::Result<()> {
    let len = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    let mut header = [0; 9];
    header[0] = kind;
    header[1..5].copy_from_slice(&id.to_le_bytes());
    header[5..9].copy_from_slice(&len.to_le_bytes());
    writer.write_all(&header)?;
    writer.write_all(payload)?;
    writer.flush()
}

pub fn pcm_bytes(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| {
            let sample = if sample.is_finite() { *sample } else { 0.0 };
            let pcm = (sample.clamp(-1.0, 1.0) * 32768.0).clamp(-32768.0, 32767.0) as i16;
            pcm.to_le_bytes()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spqx_header_golden_and_utf8_roundtrip() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, SPEAK, 0x04030201, "Grüße".as_bytes()).unwrap();
        assert_eq!(&bytes[..9], &[1, 1, 2, 3, 4, 7, 0, 0, 0]);
        let mut input = bytes.as_slice();
        let frame = Frame::read(&mut input).unwrap().unwrap();
        assert_eq!(frame.id, 0x04030201);
        assert_eq!(frame.payload, "Grüße".as_bytes());
        assert_eq!(Frame::read(&mut input).unwrap(), None);
    }

    #[test]
    fn truncation_is_not_clean_eof() {
        assert_eq!(Frame::read(&mut &[][..]).unwrap(), None);
        assert_eq!(
            Frame::read(&mut &[1, 2][..]).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        let mut bytes = Vec::new();
        write_frame(&mut bytes, SPEAK, 1, b"abc").unwrap();
        bytes.pop();
        assert_eq!(
            Frame::read(&mut bytes.as_slice()).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_oversize_before_allocating() {
        let mut header = [0; 9];
        header[5..9].copy_from_slice(&((MAX_PAYLOAD + 1) as u32).to_le_bytes());
        assert_eq!(
            Frame::read(&mut header.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn signed_pcm_is_saturated_little_endian() {
        let bytes = pcm_bytes(&[-2.0, -1.0, 0.0, 0.5, 1.0, 2.0, f32::NAN]);
        let values: Vec<_> = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert_eq!(values, [-32768, -32768, 0, 16384, 32767, 32767, 0]);
    }
}
