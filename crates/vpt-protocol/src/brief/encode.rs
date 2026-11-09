use super::*;
use serde_json::{Value, json};
use vpt_domain::brief::ContextStatus;

pub(super) fn value(document: &Document<'_>) -> Result<Value, EncodeError> {
    validate::validate(document)?;
    let pack = document.pack;
    let tasks: Vec<_> = document.tasks.iter().filter(|task| !task.completed).map(|task| json!({
        "source":task.source.as_str(),"project":task.project,"id":task.id,"content":task.content,
        "due":task.due,"completed":false,"priority":task.priority,
    })).collect();
    let participants:Vec<_> = document.occasion.participants.iter().zip(&pack.selection.participants).map(|(person,matched)| json!({
        "name":person.name,"email":person.email,"term":matched.term.as_str(),"matched_note":matched.matched_note,
    })).collect();
    let mut items = Vec::new();
    for quote in &pack.items {
        let sources = [Source {
            recording: &quote.source.recording,
            at_secs: quote.source.at_secs,
            note: &quote.source.note,
        }];
        items.push(
            serde_json::to_value(Item {
                text: &quote.text,
                certainty: Some(quote.certainty),
                sources: &sources,
                selector: quote.selector,
            })
            .map_err(|_| EncodeError::Invalid)?,
        );
    }
    Ok(json!({
        "schema":SCHEMA,
        "occasion":{"id":document.id.as_str(),"at":document.occasion.at.as_str(),"duration_secs":document.occasion.duration_secs,"title":document.occasion.title,"source":document.source.as_str(),"participants":participants},
        "unresolved":{"flags":pack.unresolved.flags,"notes_with_flags":pack.unresolved.notes_with_flags,"reviewed":pack.unresolved.reviewed},
        "items":items,"tasks":tasks,"not_selected":selection::not_selected(document),
        "selection":{"candidates":selection::candidates(&pack.selection),"omitted_notes":pack.selection.omitted_notes,"rejected_notes":pack.selection.rejected_notes,"omitted_spans":pack.omitted_spans},
        "context":{"calendar":context(&document.context.calendar),"tasks":context(&document.context.tasks)},
        "generated_at":document.generated_at.rfc3339(),
    }))
}
fn context(status: &ContextStatus) -> Value {
    json!({"status":status.status(),"reason":status.reason(),"detail":status.detail()})
}
