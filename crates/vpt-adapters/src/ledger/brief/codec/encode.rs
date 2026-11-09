use super::*;
use serde_json::{Value, json};
use vpt_domain::brief::{ContextStatus, SourceDigest};

pub(super) fn value(snapshot: &Snapshot) -> Value {
    let pack = &snapshot.pack;
    let selection = &pack.selection;
    json!({
        "schema":"vpt.saved-brief/1", "id":snapshot.id.as_str(), "source":snapshot.source.as_str(),
        "occasion":{"at":snapshot.occasion.at.as_str(),"title":snapshot.occasion.title,"duration_secs":snapshot.occasion.duration_secs,
            "participants":snapshot.occasion.participants.iter().map(|p| (&p.name,&p.email)).collect::<Vec<_>>(),"tags":snapshot.occasion.tags},
        "generated_at":snapshot.generated_at.secs,
        "context":{"calendar":half(&snapshot.context.calendar),"tasks":half(&snapshot.context.tasks)},
        "tasks":snapshot.tasks.iter().map(|t| json!({"source":t.source.as_str(),"project":t.project,"id":t.id,"content":t.content,"due":t.due,"completed":t.completed,"priority":t.priority})).collect::<Vec<_>>(),
        "pack":{
            "participants":selection.participants.iter().map(|p| json!({"participant":p.participant,"term":p.term.as_str(),"matched_note":p.matched_note,"confirmation":p.confirmation})).collect::<Vec<_>>(),
            "selected":selection.selected,
            "decisions":selection.decisions.iter().map(|d| json!({"recording":d.recording.as_str(),"selected":d.selected,"rejected":d.rejected.map(|r| r.as_str()),"matched":d.matched.as_ref().map(|m| json!({"selector":m.selector.as_str(),"values":m.values,"anchors":m.anchors.iter().map(|id| id.as_str()).collect::<Vec<_>>()}))})).collect::<Vec<_>>(),
            "omitted_notes":selection.omitted_notes,"rejected_notes":selection.rejected_notes,"omitted_spans":pack.omitted_spans,
            "items":pack.items.iter().map(|q| json!({"text":q.text,"at_secs":q.source.at_secs,"source":{"recording":q.source.recording.as_str(),"note":q.source.note,"digest":q.source.digest.hex()},"certainty":q.certainty.as_str(),"selector":q.selector.as_str()})).collect::<Vec<_>>(),
            "sources":pack.sources.iter().map(source).collect::<Vec<_>>(),
            "unresolved":{"flags":pack.unresolved.flags,"notes_with_flags":pack.unresolved.notes_with_flags,"reviewed":pack.unresolved.reviewed},
        },
    })
}
fn half(half: &ContextStatus) -> Value {
    json!({"status":half.status(),"reason":half.reason(),"detail":half.detail()})
}
fn source(source: &SourceDigest) -> Value {
    json!({"recording":source.recording.as_str(),"note":source.note,"digest":source.digest.hex()})
}
