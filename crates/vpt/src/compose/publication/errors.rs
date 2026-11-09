use crate::compose::errors::{ledger_error, store_error};
use vpt_application::RepairError;
use vpt_application::ports::StoreError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn document(error: RepairError) -> ErrorDocument {
    match error {
        RepairError::TargetModified(path) => store_error(StoreError::Changed(path)),
        RepairError::RenderedDigestMismatch(_) => ErrorDocument::new(
            ErrorKind::Refused,
            "saved publication content does not match the intended digest",
        )
        .rule("publication_digest_mismatch"),
        RepairError::Render(_) => ErrorDocument::new(
            ErrorKind::Ledger,
            "pending publication has no saved render state",
        ),
        RepairError::Sync { cause, .. } | RepairError::Stores(cause) => store_error(cause),
        RepairError::Ledger(error) => ledger_error(error),
    }
}
