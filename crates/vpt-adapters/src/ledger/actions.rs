use vpt_application::ports::{ActionChange, LedgerError, QueuedAction};

pub(super) fn check(change: &ActionChange) -> Result<(), LedgerError> {
    let valid = |action: &QueuedAction| {
        (1..=200).contains(&action.id.len())
            && !action.id.chars().any(char::is_control)
            && action.digest.len() == 64
            && action.digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let ok = match change {
        ActionChange::Queue(actions) => {
            actions.iter().all(valid)
                && actions.iter().enumerate().all(|(index, action)| {
                    actions[..index].iter().all(|earlier| {
                        (&earlier.recording, &earlier.id) != (&action.recording, &action.id)
                    })
                })
        }
        ActionChange::Settle { action, .. } => valid(action),
    };
    ok.then_some(())
        .ok_or_else(|| LedgerError::Conflict("invalid action send".into()))
}

pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("action send state conflict".into())
}
