use super::checks::check;
use crate::compose::{RuntimeLedger, errors};
use std::path::Path;
use vpt_adapters::config::planned_directory;
use vpt_adapters::{Roots, kept_files};
use vpt_application::ports::{DirtyPublication, PublicationJournal};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::Check;

pub(super) fn inspect(roots: &Roots, state: &Path) -> Check {
    let pending = pending(state);
    let mut found = Vec::new();
    for key in StoreKey::all() {
        match kept_files(
            roots.stores.get(key),
            pending.as_deref().unwrap_or_default(),
        ) {
            Ok(names) => found.extend(
                names
                    .into_iter()
                    .map(|name| format!("{}/{name}", key.key_name())),
            ),
            Err(error) => {
                let cause = errors::contained_error(error).message;
                let detail = format!(
                    "store folder {} could not be inspected: {cause}",
                    key.key_name()
                );
                return check("kept_edits", false, detail);
            }
        }
    }
    if found.is_empty() {
        return check("kept_edits", true, "none");
    }
    let plural = if found.len() == 1 { "" } else { "s" };
    let unread = match pending {
        Ok(_) => String::new(),
        Err(cause) => {
            format!("; some may be vpt's own, since its journal could not be read: {cause}")
        }
    };
    check(
        "kept_edits",
        false,
        format!(
            "{} version{plural} vpt kept beside notes: {}; keep what you need in each note, then \
             delete the kept file{unread}",
            found.len(),
            found.join(", ")
        ),
    )
}

fn pending(state: &Path) -> Result<Vec<DirtyPublication>, String> {
    let path = planned_directory(state, "home.state_dir")
        .map_err(|error| errors::root_error(&error).message)?;
    let pending = match RuntimeLedger::read_only(&path).map_err(|error| error.message)? {
        RuntimeLedger::Sqlite(ledger) => ledger.pending_publications(),
        RuntimeLedger::Empty(ledger) => ledger.pending_publications(),
    };
    pending.map_err(|error| errors::ledger_error(error).message)
}
