use crate::cli::output::Outcome;
use crate::compose::RuntimeLedger;
use crate::compose::errors::ledger_error;
use crate::compose::ledger_identity::{Observed, message, rule};
use crate::style::{self, Verdict};
use serde_json::{Value, json};
use vpt_application::ports::RecordingLedger;
use vpt_protocol::error::ErrorDocument;
use vpt_protocol::result::document;

/// Prints the same facts for `ledger show` and `ledger accept`; `preview` is absent for show.
pub(super) fn outcome(
    command: &str,
    observed: &Observed,
    preview: Option<bool>,
) -> Result<Outcome, Box<ErrorDocument>> {
    let recordings = RuntimeLedger::read_only(&observed.folder)?
        .recordings()
        .map_err(|error| Box::new(ledger_error(error)))?
        .len();
    let folder = observed.folder.display().to_string();
    let finding = observed
        .finding()
        .map(|finding| (rule(finding), message(&observed.folder, finding)));
    let mut body = json!({
        "folder": folder,
        "database": observed.ledger.as_ref().map(|ledger| json!({"id": ledger.database_id,
            "write_counter": ledger.write_counter, "recordings": recordings})),
        "this_machine": observed.marker.as_ref().map(|mark| json!({
            "database_id": mark.database_id, "write_counter": mark.write_counter,
            "write_token": mark.write_token})),
        "finding": finding.as_ref().map(|(rule, message)| json!({"rule": rule, "message": message})),
    });
    let blurb = match preview {
        None => "the record of recordings vpt keeps in this folder",
        Some(true) => "a preview; nothing changes",
        Some(false) => "this machine's next write builds on this ledger",
    };
    let mut human = style::heading(command, blurb);
    human.push_str(&style::row(Verdict::Ok, "folder", &folder));
    match &observed.ledger {
        Some(ledger) => {
            human.push_str(&style::row(Verdict::Ok, "database_id", &ledger.database_id));
            let counter = ledger.write_counter.to_string();
            human.push_str(&style::row(Verdict::Ok, "write_counter", &counter));
        }
        None => human.push_str(&style::row(
            Verdict::NotRun,
            "database_id",
            "no database yet",
        )),
    }
    human.push_str(&style::row(
        Verdict::Ok,
        "recordings",
        &recordings.to_string(),
    ));
    let machine = match &observed.marker {
        Some(mark) => format!(
            "last wrote database {} at write {}",
            mark.database_id, mark.write_counter
        ),
        None => "no write recorded yet; the next writing command records one".into(),
    };
    human.push_str(&style::row(Verdict::Ok, "this_machine", &machine));
    match &finding {
        Some((rule, message)) => human.push_str(&style::row(
            Verdict::Issue,
            "finding",
            &format!("{rule}: {message}"),
        )),
        None => human.push_str(&style::row(Verdict::Ok, "finding", "none")),
    }
    if let (Some(preview), Value::Object(fields)) = (preview, &mut body) {
        fields.insert("preview".into(), preview.into());
    }
    Ok(Outcome::Success {
        document: document(command, body),
        human,
        completed: Vec::new(),
        diagnostics: Vec::new(),
    })
}
