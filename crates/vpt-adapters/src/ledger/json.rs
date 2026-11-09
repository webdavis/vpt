use serde::Serialize;
use std::io::{self, Write};

pub(crate) fn encode(value: &impl Serialize, maximum: usize) -> Result<String, ()> {
    struct Bounded {
        bytes: Vec<u8>,
        maximum: usize,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > self.maximum - self.bytes.len() {
                return Err(io::Error::other("saved document limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Bounded {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| ())?;
    String::from_utf8(writer.bytes).map_err(|_| ())
}
