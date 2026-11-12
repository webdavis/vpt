use serde_json::{Value, json};
use std::path::Path;

pub fn event(oid: &str, subject: &str, start: i64, end: i64) -> Value {
    json!({"oid":oid,"subject":subject,"path":"calendar/work/","labels":[],
        "status":"confirmed","transparency":"busy","all_day":false,
        "start":vpt_domain::time::UtcInstant{secs:start}.rfc3339(),
        "end":vpt_domain::time::UtcInstant{secs:end}.rfc3339(),"busy":true})
}

pub fn agenda(events: Vec<Value>) -> String {
    json!({ "events": events }).to_string()
}

pub fn script(agenda: &Path) -> String {
    let shown = concat!(
        r#"{"oid":"%s","kind":"event","subject":"s","body":"","path":"calendar/work/","labels":[],"#,
        r#""depends":[],"reminders":[],"author":"owner","details":null,"event":{"start":"2026-10-01","#,
        r#""end":"2026-10-02","attendees":[{"email":"guest@example.org","response":"accepted"}],"#,
        r#""status":"confirmed","transparency":"busy","visibility":"default","event_type":"default","#,
        r#""attachments":[]}}"#,
    );
    format!(
        "case \"$1\" in\nagenda) /bin/cat {agenda:?} ;;\nshow) printf '{shown}' \"$2\" ;;\nls) printf '%s' '{{\"objects\":[]}}' ;;\n*) exit 8 ;;\nesac\n"
    )
}
