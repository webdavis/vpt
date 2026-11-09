use crate::{identity::RecordingId, time::UtcInstant};

#[derive(Debug, Clone, Copy)]
pub struct Capture<'a> {
    pub id: &'a RecordingId,
    pub at: UtcInstant,
    pub duration_secs: u64,
}

pub fn continues(
    current: Capture<'_>,
    recordings: &[Capture<'_>],
    session_gap_minutes: u32,
) -> Option<RecordingId> {
    let previous = recordings
        .iter()
        .filter(|row| {
            row.id != current.id && (row.at, row.id.as_str()) < (current.at, current.id.as_str())
        })
        .max_by(|left, right| (left.at, left.id.as_str()).cmp(&(right.at, right.id.as_str())))?;
    let end = i128::from(previous.at.secs) + i128::from(previous.duration_secs);
    let gap = i128::from(current.at.secs) - end;
    (gap > 0 && gap <= i128::from(session_gap_minutes) * 60).then(|| previous.id.clone())
}

#[cfg(test)]
mod tests;
