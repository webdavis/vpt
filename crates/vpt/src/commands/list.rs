use crate::cli::output::Outcome;
use crate::documents::observation_json;
use serde_json::json;
use vpt_application::ports::{ObservationLedger, RecordingRecord, StageState};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub(crate) fn run(ledger: &dyn ObservationLedger, stage: Option<&str>) -> Outcome {
    let outstanding: fn(&RecordingRecord) -> bool = match stage {
        None => |_| true,
        Some("transcribe") => |record| record.stages.transcribe != StageState::Succeeded,
        Some("note") => |record| record.stages.note != StageState::Succeeded,
        Some("synthesis") => |record| record.stages.synthesis != StageState::Succeeded,
        Some(_) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid stage")),
    };
    let records = match ledger.observe_recordings(None) {
        Ok(records) => records,
        Err(error) => return Outcome::Failure(crate::compose::errors::ledger_error(error)),
    };
    let selected: Vec<_> = records
        .iter()
        .filter(|record| outstanding(&record.record))
        .collect();
    let rows: Vec<_> = match selected.iter().copied().map(observation_json).collect() {
        Ok(rows) => rows,
        Err(error) => return Outcome::Failure(error),
    };
    let human: String = selected
        .iter()
        .map(|record| {
            let record = &record.record;
            let title = record.title.as_deref().unwrap_or("(untitled)");
            format!(
                "{}  {}  [transcribe: {}, note: {}, synthesis: {}]\n",
                record.id,
                title.escape_debug(),
                record.stages.transcribe.as_str(),
                record.stages.note.as_str(),
                record.stages.synthesis.as_str()
            )
        })
        .collect();
    Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        human: if human.is_empty() {
            "no recordings\n".into()
        } else {
            human
        },
        document: document("list", json!({"recordings": rows})),
    }
}
