use super::checks::Context;
use super::report::Row;
use crate::compose::{RuntimeLedger, errors};
use vpt_adapters::config::planned_directory;
use vpt_application::ports::{LedgerError, RecordingLedger, SeenRow};

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let folder = planned_directory(&context.settings.state_dir, "home.state_dir")
        .map_err(|error| errors::root_error(&error).message);
    let opened = folder
        .clone()
        .and_then(|path| RuntimeLedger::read_only(&path).map_err(|error| error.message));
    let ledger = match opened {
        Ok(ledger) => ledger,
        Err(message) => {
            return vec![
                Row::issue("database", message),
                Row::not_run("schema", "the ledger did not open"),
                Row::not_run("source_gone", "the ledger did not open"),
            ];
        }
    };
    let (database, schema) = match &ledger {
        RuntimeLedger::Empty(_) => (
            Row::ok("database", "none yet; the first `vpt run` creates it"),
            Row::not_run("schema", "no ledger yet"),
        ),
        RuntimeLedger::Sqlite(sqlite) => (
            Row::ok(
                "database",
                format!("opens read-only at {}", sqlite.database_path().display()),
            ),
            match sqlite.schema_version() {
                Ok(version) => Row::ok("schema", version.to_string()),
                Err(error) => Row::issue("schema", errors::ledger_error(error).message),
            },
        ),
    };
    let mut rows = vec![database, schema, source_gone(ledger.seen_all())];
    if let Ok(folder) = &folder {
        rows.extend(super::ledger_identity::rows(
            folder,
            context.markers.as_ref(),
        ));
    }
    rows
}

fn source_gone(rows: Result<Vec<SeenRow>, LedgerError>) -> Row {
    match rows {
        Ok(rows) => Row::ok(
            "source_gone",
            format!(
                "{} recordings whose source is gone",
                rows.iter()
                    .filter(|row| row.source_gone_at.is_some())
                    .count()
            ),
        ),
        Err(error) => Row::issue("source_gone", errors::ledger_error(error).message),
    }
}

#[cfg(test)]
#[path = "ledger/tests.rs"]
mod tests;
