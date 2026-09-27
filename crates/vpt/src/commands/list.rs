use crate::cli::output::Outcome;
use crate::documents::record_json;
use serde_json::json;
use vpt_application::ports::{RecordingLedger, RecordingRecord, StageState};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub(crate) fn run(ledger: &dyn RecordingLedger, stage: Option<&str>) -> Outcome {
    let outstanding: fn(&RecordingRecord) -> bool = match stage {
        None => |_| true,
        Some("transcribe") => |record| record.stages.transcribe != StageState::Succeeded,
        Some("note") => |record| record.stages.note != StageState::Succeeded,
        Some("synthesis") => |record| record.stages.synthesis != StageState::Succeeded,
        Some(_) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid stage")),
    };
    let records = match ledger.recordings() {
        Ok(records) => records,
        Err(error) => return Outcome::Failure(crate::compose::errors::ledger_error(error)),
    };
    let selected: Vec<_> = records
        .iter()
        .filter(|record| outstanding(record))
        .collect();
    let rows: Vec<_> = match selected.iter().copied().map(record_json).collect() {
        Ok(rows) => rows,
        Err(error) => return Outcome::Failure(error),
    };
    let human: String = selected
        .iter()
        .map(|record| {
            let title = record.title.as_deref().unwrap_or("(untitled)");
            format!("{}  {}\n", record.id, title.escape_debug())
        })
        .collect();
    Outcome::Success {
        human: if human.is_empty() {
            "no recordings\n".into()
        } else {
            human
        },
        document: document("list", json!({"recordings": rows})),
    }
}
