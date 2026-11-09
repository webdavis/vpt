#[derive(Clone, Copy)]
pub(crate) enum EncodeError {
    Invalid,
    CapacityExceeded,
}
use std::io::{self, Write};

pub(super) fn encode(value: &serde_json::Value) -> Result<String, EncodeError> {
    struct Output {
        bytes: Vec<u8>,
        full: bool,
    }
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > 32 * 1024 * 1024 - self.bytes.len() {
                self.full = true;
                return Err(io::Error::other("document output capacity"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output {
        bytes: vec![],
        full: false,
    };
    serde_json::to_writer(&mut output, value).map_err(|_| {
        if output.full {
            EncodeError::CapacityExceeded
        } else {
            EncodeError::Invalid
        }
    })?;
    String::from_utf8(output.bytes).map_err(|_| EncodeError::Invalid)
}
