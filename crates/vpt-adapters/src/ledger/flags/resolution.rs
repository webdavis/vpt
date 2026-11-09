use vpt_application::ports::{FlagResolutionChange, LedgerError, WorkDelta};
use vpt_domain::{
    review::{Resolution, ReviewAction, ReviewFlag},
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(in crate::ledger) fn conflict() -> LedgerError {
    LedgerError::Conflict("flag resolution conflict".into())
}

pub(in crate::ledger) fn check_batch(
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    if delta.resolution.is_some() && delta.transcription.is_some() {
        return Err(conflict());
    }
    for (_, fact) in facts {
        if let HistoryFact::FlagResolved {
            recording,
            flag,
            resolution,
        } = fact
            && delta.resolution.as_ref().is_none_or(|change| {
                change.recording != *recording
                    || change.flag != *flag
                    || change.resolution.kind() != Some(*resolution)
            })
        {
            return Err(conflict());
        }
    }
    Ok(())
}

pub(in crate::ledger) fn check(
    change: &FlagResolutionChange,
    flag: &ReviewFlag,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    if flag.identifier != change.flag
        || flag.resolution != change.expected
        || change.expected == change.resolution
        || !change.resolution.valid_text()
        || !change.expected.valid_text()
    {
        return Err(conflict());
    }
    let at = change.resolution.at().ok_or_else(conflict)?;
    let action = match &change.resolution {
        Resolution::Confirmed { .. } => ReviewAction::Confirm,
        Resolution::Corrected { text, .. } => ReviewAction::Correct(text.clone()),
        Resolution::Dismissed { .. } => ReviewAction::Dismiss,
        Resolution::Open => return Err(conflict()),
    };
    if flag.resolve(&action, at).map_err(|_| conflict())? != change.resolution {
        return Err(conflict());
    }
    let work: Vec<_> = facts
        .iter()
        .filter(|(_, fact)| {
            matches!(fact,
        HistoryFact::WorkStep { recording: Some(id), .. } if id == &change.recording)
        })
        .collect();
    let resolved: Vec<_> = facts
        .iter()
        .filter(|(_, fact)| {
            matches!(fact,
        HistoryFact::FlagResolved { recording, flag, resolution } if recording == &change.recording
            && flag == &change.flag && Some(*resolution) == change.resolution.kind())
        })
        .collect();
    if work.len() != 1
        || work[0].0 != at
        || !matches!(
            work[0].1,
            HistoryFact::WorkStep {
                outcome: WorkOutcome::Completed,
                ..
            }
        )
        || resolved.len() != 1
        || resolved[0].0 != at
    {
        return Err(conflict());
    }
    Ok(())
}
