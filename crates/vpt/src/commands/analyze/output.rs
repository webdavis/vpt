use crate::{cli::output::Outcome, commands::note, compose::errors::ledger_error};
use vpt_application::ports::*;
use vpt_domain::{identity::RecordingId, review::FlagContent};
use vpt_protocol::error::ErrorDocument;

pub(super) fn success(
    ledger: &impl NoteInputsLedger,
    id: &RecordingId,
    diagnostics: &[String],
) -> Result<Outcome, Box<ErrorDocument>> {
    let input = ledger
        .note_inputs(id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    let mut verify = std::collections::BTreeMap::new();
    for flag in &input.review.flags {
        if let FlagContent::Verification(finding) = &flag.content
            && finding.artifact == NoteStage::Analysis
        {
            *verify.entry(finding.finding.as_str()).or_insert(0usize) += 1;
        }
    }
    let record = crate::documents::record_json(&input.record).map_err(Box::new)?;
    let paths: Vec<_> = input.notes.iter().map(|note| &note.target).collect();
    let details: String = diagnostics
        .iter()
        .map(|detail| format!("Diagnostic: {detail}\n"))
        .collect();
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: diagnostics.to_vec(),
        document: vpt_protocol::result::document(
            "analyze",
            serde_json::json!({"record": record, "paths": paths, "verify": verify, "diagnostics": diagnostics}),
        ),
        human: format!(
            "Analyzed {id}; {} findings\n{details}",
            verify.values().sum::<usize>()
        ),
    })
}
