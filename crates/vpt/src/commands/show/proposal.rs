use super::errors;
use serde_json::{Value, json};
use vpt_application::ports::NoteInputs;
use vpt_domain::{identity::RecordingId, proposal::CheckedClaim};
use vpt_protocol::error::ErrorDocument;

pub(super) fn json(input: &NoteInputs, body: &mut Value) -> Result<(), Box<ErrorDocument>> {
    let accepted = input.proposal.as_ref().ok_or_else(errors::missing)?;
    let lines = |claims: &[CheckedClaim]| -> Vec<Value> {
        claims
            .iter()
            .map(|claim| line(&input.record.id, claim))
            .collect()
    };
    body["summary"] = json!(lines(&accepted.proposal.summary));
    body["actions"] = json!(lines(&accepted.proposal.actions));
    Ok(())
}

fn line(id: &RecordingId, claim: &CheckedClaim) -> Value {
    let sources = &claim.grounding.sources;
    json!({
        "moment": sources
            .first()
            .map(|source| format!("{}@{}", id.as_str(), super::timecode(source.start_secs))),
        "text": claim.claim.text,
        "sources": sources
            .iter()
            .map(|source| json!({
                "start": source.start_secs,
                "end": source.end_secs,
                "range": format!(
                    "{}-{}",
                    super::timecode(source.start_secs),
                    super::timecode(source.end_secs)
                ),
            }))
            .collect::<Vec<_>>(),
        "finding": claim.grounding.finding.as_ref().map(|finding| finding.as_str()),
    })
}
