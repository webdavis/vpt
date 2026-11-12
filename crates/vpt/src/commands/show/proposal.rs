use super::errors;
use serde_json::{Value, json};
use vpt_application::ports::NoteInputs;
use vpt_domain::{
    grounding::SourceRange,
    identity::RecordingId,
    proposal::{CheckedAction, CheckedClaim, CheckedDue, Owner},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn json(input: &NoteInputs, body: &mut Value) -> Result<(), Box<ErrorDocument>> {
    let accepted = input.proposal.as_ref().ok_or_else(errors::missing)?;
    let id = &input.record.id;
    let summary: Vec<_> = accepted
        .proposal
        .summary
        .iter()
        .map(|claim| line(id, claim))
        .collect();
    let actions: Vec<_> = accepted
        .proposal
        .actions
        .iter()
        .map(|action| action_line(id, action))
        .collect();
    body["summary"] = json!(summary);
    body["actions"] = json!(actions);
    Ok(())
}

fn action_line(id: &RecordingId, action: &CheckedAction) -> Value {
    let mut line = line(id, &action.claim);
    line["owner"] = match &action.owner {
        None => Value::Null,
        Some(Owner::Me) => json!("me"),
        Some(Owner::Other(name)) => json!({ "other": name }),
    };
    line["due"] = action.due.as_ref().map_or(Value::Null, due);
    line
}

fn due(due: &CheckedDue) -> Value {
    let shown = !due.withheld();
    json!({
        "date": shown.then_some(&due.due.date),
        "time": due.due.time.as_ref().filter(|_| shown),
        "sources": sources(&due.grounding.sources),
        "finding": due.grounding.finding.as_ref().map(|finding| finding.as_str()),
    })
}

fn line(id: &RecordingId, claim: &CheckedClaim) -> Value {
    let sources = &claim.grounding.sources;
    json!({
        "moment": sources
            .first()
            .map(|source| format!("{}@{}", id.as_str(), super::timecode(source.start_secs))),
        "text": claim.claim.text,
        "sources": self::sources(sources),
        "finding": claim.grounding.finding.as_ref().map(|finding| finding.as_str()),
    })
}

fn sources(sources: &[SourceRange]) -> Vec<Value> {
    sources
        .iter()
        .map(|source| {
            json!({
                "start": source.start_secs,
                "end": source.end_secs,
                "range": format!(
                    "{}-{}",
                    super::timecode(source.start_secs),
                    super::timecode(source.end_secs)
                ),
            })
        })
        .collect()
}
