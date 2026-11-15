use super::checks::Context;
use super::report::Row;
use crate::compose::{RuntimeLedger, errors};
use std::path::Path;
use vpt_adapters::config::planned_directory;
use vpt_adapters::{ClonefileArchive, Roots, kept_files};
use vpt_application::ports::{Archive, DirtyPublication, PublicationJournal};
use vpt_domain::layout::StoreKey;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    match &context.roots {
        Ok(roots) => vec![
            staged_audio(&roots.stores.audio),
            kept_edits(roots, &context.settings.state_dir),
        ],
        Err(_) => ["staged_audio", "kept_edits"]
            .map(|name| Row::not_run(name, "the store folders did not resolve"))
            .into(),
    }
}

fn staged_audio(audio: &Path) -> Row {
    let leftovers = ClonefileArchive::open_read_only(audio)
        .map_err(|error| errors::contained_error(error).message)
        .and_then(|archive| {
            archive
                .staged_leftovers()
                .map_err(|_| "staged files could not be inspected".to_string())
        });
    match leftovers {
        Ok(leftovers) if leftovers.is_empty() => Row::ok("staged_audio", "none"),
        Ok(leftovers) => {
            let names: Vec<_> = leftovers
                .iter()
                .map(|path| path.display().to_string())
                .collect();
            Row::issue(
                "staged_audio",
                format!(
                    "{} staged files await the Trash: {}",
                    names.len(),
                    names.join(", ")
                ),
            )
            .advice("move them to the Trash once no vpt command is running")
        }
        Err(message) => Row::issue("staged_audio", message),
    }
}

fn kept_edits(roots: &Roots, state: &Path) -> Row {
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
                return Row::issue("kept_edits", detail);
            }
        }
    }
    if found.is_empty() {
        return Row::ok("kept_edits", "none");
    }
    let plural = if found.len() == 1 { "" } else { "s" };
    let unread = match pending {
        Ok(_) => String::new(),
        Err(cause) => {
            format!("; some may be vpt's own, since its journal could not be read: {cause}")
        }
    };
    Row::issue(
        "kept_edits",
        format!(
            "{} version{plural} vpt kept beside notes: {}{unread}",
            found.len(),
            found.join(", ")
        ),
    )
    .advice("keep what you need in each note, then delete the kept file")
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
