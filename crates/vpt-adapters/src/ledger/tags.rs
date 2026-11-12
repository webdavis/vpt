use std::collections::HashSet;
use vpt_application::ports::{LedgerError, TagChange, TagRename};
use vpt_domain::{
    run::{HistoryFact, WorkOutcome},
    tags::{Provenance, Tag},
    time::UtcInstant,
};
pub(super) mod codec;

pub(super) const MAX_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved tags".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("tag state conflict".into())
}

pub(super) fn validate(tags: &[Tag]) -> Result<(), LedgerError> {
    if tags.len() > 256 {
        return Err(invalid());
    }
    let mut keys = HashSet::new();
    let mut remaining = MAX_BYTES;
    for tag in tags {
        let key = vpt_domain::tags::key(&tag.name).map_err(|_| invalid())?;
        if !keys.insert(key) {
            return Err(invalid());
        }
        let mut fields = vec![tag.name.as_str()];
        if let Provenance::Proposal { by, reason, .. } = &tag.provenance {
            if by.trim().is_empty() {
                return Err(invalid());
            }
            fields.extend([by.as_str(), reason.as_str()]);
        }
        for text in fields {
            if text.len() > 4 * 65_536
                || text.chars().count() > 65_536
                || text
                    .chars()
                    .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\n' | '\r'))
            {
                return Err(invalid());
            }
            remaining = remaining.checked_sub(text.len()).ok_or_else(invalid)?;
        }
    }
    Ok(())
}

pub(super) fn check(
    change: &TagChange,
    current: &Option<Vec<Tag>>,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validate(&change.tags)?;
    if &change.expected != current || facts.iter().filter(|(at, fact)|
        *at == change.at && matches!(fact, HistoryFact::WorkStep { recording: Some(id), outcome: WorkOutcome::Completed } if *id == change.recording)
    ).count() != 1 { return Err(conflict()); }
    Ok(())
}

pub(super) fn check_rename(rename: &TagRename) -> Result<(), LedgerError> {
    let old = vpt_domain::tags::key(&rename.old).map_err(|_| invalid())?;
    let new = vpt_domain::tags::key(&rename.new).map_err(|_| invalid())?;
    if old != rename.old || old == new || rename.new.len() > 262_144 {
        return Err(invalid());
    }
    Ok(())
}
