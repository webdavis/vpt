pub(super) mod basis;
pub(super) mod codec;
mod current;
mod validate;
pub(super) use current::check as is_current;
pub(super) use validate::{check, history};
use vpt_application::ports::LedgerError;

pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid accepted proposal".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("proposal state conflict".into())
}
