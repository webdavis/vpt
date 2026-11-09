pub(crate) mod change;
pub(crate) mod codec;
pub(crate) mod published;
pub(crate) mod reservations;
pub(crate) mod validate;
use vpt_application::ports::LedgerError;

pub(crate) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved brief".into())
}
pub(crate) fn conflict() -> LedgerError {
    LedgerError::Conflict("brief publication conflict".into())
}

pub fn prepare(
    snapshot: vpt_domain::brief::document::Snapshot,
    target: std::path::PathBuf,
    text: String,
    managed_tags: Vec<String>,
    first_written: vpt_domain::time::UtcInstant,
    write_offset: vpt_domain::time::UtcOffset,
) -> Result<vpt_application::ports::BriefArtifact, LedgerError> {
    let encoded = codec::encode(&snapshot)?;
    let artifact = vpt_application::ports::BriefArtifact {
        version: vpt_application::ports::BriefVersion {
            content: validate::digest(text.as_bytes()),
            snapshot: validate::digest(encoded.as_bytes()),
        },
        snapshot,
        target,
        text,
        managed_tags,
        first_written,
        write_offset,
        published_at: None,
    };
    validate::validate(&artifact)?;
    Ok(artifact)
}

pub(crate) fn check_facts(
    delta: &vpt_application::ports::WorkDelta,
    facts: &[(vpt_domain::time::UtcInstant, vpt_domain::run::HistoryFact)],
) -> Result<(), LedgerError> {
    use vpt_domain::run::HistoryFact;
    if facts.iter().any(|(_, fact)| match fact {
        HistoryFact::BriefSaved { .. } => delta.brief.is_none(),
        HistoryFact::BriefPublished { .. } => delta.brief_published.is_none(),
        _ => false,
    }) {
        return Err(conflict());
    }
    Ok(())
}
