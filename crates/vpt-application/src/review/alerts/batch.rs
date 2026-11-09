use super::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertError {
    CapacityExceeded,
    DuplicateRecording,
}

pub fn events(
    notices: &[ReviewNotice],
    aggregate_after: u32,
    at: UtcInstant,
) -> Result<Vec<Event>, AlertError> {
    if notices.len() > 65_536 {
        return Err(AlertError::CapacityExceeded);
    }
    let mut remaining = 16 * 1024 * 1024usize;
    let mut identities = HashSet::new();
    let mut counts = EventCounts::default();
    for notice in notices {
        remaining = remaining
            .checked_sub(notice.path.as_os_str().len())
            .ok_or(AlertError::CapacityExceeded)?;
        if !identities.insert(notice.recording.as_str()) {
            return Err(AlertError::DuplicateRecording);
        }
        add(&mut counts, &notice.counts)?;
    }
    if notices.len() <= aggregate_after as usize {
        return Ok(notices.iter().map(|notice| notice.event(at)).collect());
    }
    Ok(vec![Event {
        kind: EventKind::ReviewNeeded,
        state: EventState::NeedsAttention,
        occasion: None,
        recording: None,
        paths: vec![],
        counts,
        occurred_at: at,
        detail: EventDetail::ReviewAggregate {
            recordings: notices.len() as u32,
        },
    }])
}

fn add(target: &mut EventCounts, value: &EventCounts) -> Result<(), AlertError> {
    for (target, value) in [
        (&mut target.numeric, value.numeric),
        (&mut target.proper_noun, value.proper_noun),
        (&mut target.low_confidence, value.low_confidence),
        (&mut target.formatting, value.formatting),
        (&mut target.other, value.other),
        (&mut target.unsupported, value.unsupported),
        (&mut target.unsourced, value.unsourced),
        (&mut target.bad_reference, value.bad_reference),
        (
            &mut target.built_on_flagged_text,
            value.built_on_flagged_text,
        ),
        (&mut target.agreed_unverified, value.agreed_unverified),
        (&mut target.diagnostics, value.diagnostics),
    ] {
        *target = target
            .checked_add(value)
            .ok_or(AlertError::CapacityExceeded)?;
    }
    Ok(())
}
