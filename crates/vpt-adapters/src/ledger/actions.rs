use vpt_application::ports::{ActionChange, LedgerError, QueuedAction};

pub(super) fn check(change: &ActionChange) -> Result<(), LedgerError> {
    let valid_id = |id: &str| (1..=200).contains(&id.len()) && !id.chars().any(char::is_control);
    let valid = |action: &QueuedAction| {
        valid_id(&action.id)
            && action.digest.len() == 64
            && action.digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let ok = match change {
        ActionChange::Queue {
            recording,
            actions,
            dropped,
        } => {
            let ids: Vec<&str> = actions
                .iter()
                .map(|action| action.id.as_str())
                .chain(dropped.iter().map(String::as_str))
                .collect();
            actions
                .iter()
                .all(|action| valid(action) && action.recording == *recording)
                && dropped.iter().all(|id| valid_id(id))
                && ids
                    .iter()
                    .enumerate()
                    .all(|(index, id)| !ids[..index].contains(id))
        }
        ActionChange::Attempt(action)
        | ActionChange::Settle { action, .. }
        | ActionChange::Withdrawn(action) => valid(action),
        ActionChange::ResendRefused(_) => true,
    };
    ok.then_some(())
        .ok_or_else(|| LedgerError::Conflict("invalid action send".into()))
}

pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("action send state conflict".into())
}
