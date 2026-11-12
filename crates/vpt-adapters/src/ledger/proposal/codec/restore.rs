use super::{grounding, invalid};
use vpt_application::ports::LedgerError;
use vpt_domain::proposal::{Action, CheckedAction, CheckedClaim, CheckedDue, Claim};

pub(super) fn claims(
    claims: Vec<Claim>,
    saved: Vec<grounding::SavedGrounding>,
) -> Result<Vec<CheckedClaim>, LedgerError> {
    if claims.len() != saved.len() {
        return Err(invalid());
    }
    claims
        .into_iter()
        .zip(saved)
        .map(|(claim, saved)| {
            Ok(CheckedClaim {
                claim,
                grounding: grounding::restore(saved)?,
            })
        })
        .collect()
}

pub(super) fn action(
    action: Action,
    mut saved: grounding::SavedGrounding,
) -> Result<CheckedAction, LedgerError> {
    let due = match (action.due, saved.due.take()) {
        (Some(due), Some(grounding)) => Some(CheckedDue {
            due,
            grounding: grounding::restore(*grounding)?,
        }),
        (None, None) => None,
        _ => return Err(invalid()),
    };
    Ok(CheckedAction {
        claim: CheckedClaim {
            claim: action.claim,
            grounding: grounding::restore(saved)?,
        },
        owner: action.owner,
        due,
    })
}
