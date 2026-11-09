use super::*;
use serde_json::{Value, json};
use vpt_domain::brief::{Rejection, Selection, TermState};

pub(super) fn candidates(selection: &Selection) -> Value {
    Value::Array(selection.decisions.iter().map(|d| json!({
        "recording":d.recording.as_str(),"selected":d.selected,
        "selector":d.matched.as_ref().map(|m| m.selector.as_str()),
        "matched":d.matched.as_ref().map(|m| &m.values),
        "anchors":d.matched.as_ref().map(|m| m.anchors.iter().map(|id| id.as_str()).collect::<Vec<_>>()),
        "reason":d.rejected.map(Rejection::as_str).or_else(|| (!d.selected).then_some("note-limit")),
    })).collect())
}
pub(super) fn not_selected(document: &Document<'_>) -> Value {
    let mut values = Vec::new();
    for matched in &document.pack.selection.participants {
        if matched.term != TermState::Confirmed {
            let participant = &document.occasion.participants[matched.participant];
            values.push(json!({"participant":participant.name,"reason":matched.term.as_str(),"command":matched.confirmation}));
        }
    }
    for decision in &document.pack.selection.decisions {
        if !decision.selected {
            values.push(json!({"recording":decision.recording.as_str(),"reason":decision.rejected.map(Rejection::as_str).unwrap_or("note-limit")}));
        }
    }
    if document.pack.omitted_spans > 0 {
        values.push(json!({"reason":"span-limit","count":document.pack.omitted_spans}));
    }
    Value::Array(values)
}
