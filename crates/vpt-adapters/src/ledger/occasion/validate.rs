use super::digest;
use vpt_application::ports::{LedgerError, OccasionChange, OccasionRecord};
use vpt_domain::{
    occasion::{OccasionId, OccasionKey, OccasionSource},
    run::HistoryFact,
    time::UtcInstant,
};

pub(crate) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved occasion".into())
}
pub(crate) fn conflict() -> LedgerError {
    LedgerError::Conflict("occasion state conflict".into())
}

pub(crate) fn validate(row: &OccasionRecord) -> Result<(), LedgerError> {
    let expected_id = OccasionId::derive(&row.details.at, &digest(&row.key));
    if !row.details.is_valid() || row.id.hash12() != expected_id.hash12() {
        return Err(invalid());
    }
    if row.key.source() == OccasionSource::Manual
        && (row.id != expected_id
            || OccasionKey::manual(&row.details.at, &row.details.title)
                .ok()
                .as_ref()
                != Some(&row.key))
    {
        return Err(invalid());
    }
    if row.briefed_at.is_some_and(|at| {
        !(0..=9999).contains(&at.civil(vpt_domain::time::UtcOffset { secs: 0 }).year)
    }) || (row.briefed_at.is_some() && row.brief_path.is_none())
    {
        return Err(invalid());
    }
    if let Some(path) = &row.brief_path {
        let text = path.to_str().ok_or_else(invalid)?;
        if text.len() > 65_536
            || text.contains('\0')
            || !path.is_absolute()
            || path.components().any(|c| {
                !matches!(
                    c,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
            || text
                .split('/')
                .skip(1)
                .any(|part| matches!(part, "" | "." | ".."))
            || !text.ends_with(".md")
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(crate) fn check(
    change: &OccasionChange,
    current: &Option<OccasionRecord>,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validate(&change.occasion)?;
    let matching = facts
        .iter()
        .filter(|(_, fact)| matches!(fact, HistoryFact::OccasionSaved { .. }))
        .collect::<Vec<_>>();
    if &change.expected != current
        || matching.as_slice()
            != [&(
                change.at,
                HistoryFact::OccasionSaved {
                    occasion: change.occasion.id.clone(),
                },
            )]
    {
        return Err(conflict());
    }
    let row = &change.occasion;
    match current {
        Some(old)
            if old.id == row.id
                && old.key == row.key
                && old.brief_path == row.brief_path
                && old.briefed_at == row.briefed_at =>
        {
            Ok(())
        }
        None if row.brief_path.is_none()
            && row.briefed_at.is_none()
            && row.id == OccasionId::derive(&row.details.at, &digest(&row.key)) =>
        {
            Ok(())
        }
        _ => Err(conflict()),
    }
}
