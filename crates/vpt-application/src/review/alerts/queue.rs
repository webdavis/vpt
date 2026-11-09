use super::*;
use std::collections::HashSet;

#[derive(Default)]
pub struct ReviewQueue {
    notices: Vec<ReviewNotice>,
    identities: HashSet<RecordingId>,
    path_bytes: usize,
}
impl ReviewQueue {
    pub fn replace(&mut self, notice: ReviewNotice) -> Result<(), AlertError> {
        if !self.identities.contains(&notice.recording) {
            return self.push(notice);
        }
        if let Some(previous) = self
            .notices
            .iter_mut()
            .rev()
            .find(|row| row.recording == notice.recording)
        {
            let bytes = self.path_bytes - previous.path.as_os_str().len();
            let bytes = bytes
                .checked_add(notice.path.as_os_str().len())
                .filter(|bytes| *bytes <= 16 * 1024 * 1024)
                .ok_or(AlertError::CapacityExceeded)?;
            self.path_bytes = bytes;
            *previous = notice;
            return Ok(());
        }
        self.push(notice)
    }
    pub fn remove(&mut self, recording: &RecordingId) {
        if !self.identities.remove(recording) {
            return;
        }
        if let Some(index) = self
            .notices
            .iter()
            .rposition(|row| &row.recording == recording)
        {
            let previous = self.notices.remove(index);
            self.path_bytes -= previous.path.as_os_str().len();
        }
    }
    pub fn push(&mut self, notice: ReviewNotice) -> Result<(), AlertError> {
        let bytes = self
            .path_bytes
            .checked_add(notice.path.as_os_str().len())
            .ok_or(AlertError::CapacityExceeded)?;
        if self.notices.len() == 65_536 || bytes > 16 * 1024 * 1024 {
            return Err(AlertError::CapacityExceeded);
        }
        if !self.identities.insert(notice.recording.clone()) {
            return Err(AlertError::DuplicateRecording);
        }
        self.path_bytes = bytes;
        self.notices.push(notice);
        Ok(())
    }
    pub fn events(&self, aggregate_after: u32, at: UtcInstant) -> Result<Vec<Event>, AlertError> {
        events(&self.notices, aggregate_after, at)
    }
}
