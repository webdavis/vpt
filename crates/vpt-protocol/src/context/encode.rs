mod model;
use super::{DecodeError, SCHEMA};
use crate::LimitViolation;
use std::io::{self, Write};
use vpt_domain::context::ContextSnapshot;

pub(super) fn encode(snapshot: &ContextSnapshot) -> Result<String, DecodeError> {
    if snapshot.occasions.len() > 256
        || snapshot.tasks.len() > 256
        || snapshot
            .occasions
            .iter()
            .any(|row| row.details.participants.len() > 256 || row.details.tags.len() > 256)
    {
        return Err(DecodeError::Limit(LimitViolation::Array(257)));
    }
    let value = model::Document::new(snapshot);
    let mut output = Output(Vec::new());
    serde_json::to_writer(&mut output, &value)
        .map_err(|_| DecodeError::Limit(LimitViolation::Bytes(65_537)))?;
    String::from_utf8(output.0).map_err(|_| DecodeError::Invalid("document"))
}

struct Output(Vec<u8>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > 65_536 - self.0.len() {
            return Err(io::Error::other("context output capacity"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
