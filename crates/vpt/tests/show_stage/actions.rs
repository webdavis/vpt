use super::*;
use serde_json::Value;
use vpt_domain::identity::RecordingId;

fn synthesized(name: &str, actions: Value) -> (Sandbox, RecordingId, Value, String) {
    let sandbox = fixture::configured(name);
    let id = recording::seed_flags(&sandbox, false);
    let script = sandbox.path().join("agent.sh");
    let proposal = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    let document = json!({"schema": "vpt.proposal/1", "by": "fixture", "summary": [{"text": "meet Alice", "sources": ["00:00-00:01"]}], "actions": actions, "tags": [], "relations": []});
    fs::write(&proposal, document.to_string()).unwrap();
    fixture::write_config(
        &sandbox,
        &format!("[synthesis]\ncommand=['/bin/sh', {script:?}, {proposal:?}]\n"),
    );
    json(&invoke(&sandbox, &["synthesize", id.as_str(), "--json"]));
    let shown = json(&show(&sandbox, id.as_str(), "analysis"));
    let note = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .note(&id, NoteStage::Analysis)
        .unwrap()
        .unwrap()
        .text;
    (sandbox, id, shown, actions_section(&note))
}

fn actions_section(note: &str) -> String {
    let start = note.find("## Actions\n").unwrap();
    let lines: Vec<_> = note[start..]
        .lines()
        .take_while(|line| line.starts_with('#') || line.starts_with("- ") || line.is_empty())
        .collect();
    format!("{}\n", lines.join("\n").trim_end())
}

fn action(text: &str, extra: Value) -> Value {
    let mut action = json!({"text": text, "sources": ["00:00-00:01"]});
    action
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    action
}

#[test]
fn each_action_carries_whose_it_is_to_show_and_the_note() {
    let (_sandbox, id, shown, section) = synthesized(
        "show-action-owner",
        json!([
            action("email Bob", json!({"owner": "me"})),
            action("call Alice", json!({"owner": {"other": "Alice"}})),
            action("check in", json!({"owner": {"other": null}})),
            action("send notes", json!({})),
        ]),
    );
    let owners: Vec<_> = shown["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["owner"].clone())
        .collect();
    assert_eq!(
        owners,
        [
            json!("me"),
            json!({"other": "Alice"}),
            json!({"other": null}),
            json!(null)
        ]
    );
    for line in shown["actions"].as_array().unwrap() {
        assert_eq!(line["moment"], format!("{}@00:00", id.as_str()));
        assert_eq!(line["finding"], json!(null));
    }
    assert!(shown["summary"][0].get("owner").is_none());
    assert_eq!(
        section,
        "## Actions\n\n- email Bob [00:00-00:01] [owner:: me]\n- call Alice [00:00-00:01] [owner:: Alice]\n- check in [00:00-00:01] [owner:: someone else]\n- send notes [00:00-00:01]\n"
    );
}

#[test]
fn actions_without_owner_or_due_render_exactly_as_before() {
    let (_sandbox, _id, shown, section) = synthesized(
        "show-action-plain",
        json!([action("send notes", json!({})), {"text": "call Alice", "sources": []}]),
    );
    assert_eq!(
        section,
        "## Actions\n\n- send notes [00:00-00:01]\n- call Alice [unsourced]\n"
    );
    assert_eq!(shown["actions"][1]["owner"], json!(null));
    assert_eq!(shown["actions"][1]["due"], json!(null));
}

#[test]
fn an_owner_name_cannot_inject_markdown_or_frontmatter() {
    let name = "Dana] [due:: 2030-01-01] *x*\n---\ntitle: y #tag";
    let (_sandbox, _id, shown, section) = synthesized(
        "show-action-owner-escaped",
        json!([action("send notes", json!({"owner": {"other": name}}))]),
    );
    assert_eq!(shown["actions"][0]["owner"], json!({"other": name}));
    assert_eq!(shown["actions"][0]["finding"], json!(null));
    assert_eq!(
        section,
        "## Actions\n\n- send notes [00:00-00:01] [owner:: Dana&#93; &#91;due&#58;&#58; 2030-01-01&#93; &#42;x&#42;&#10;---&#10;title&#58; y &#35;tag]\n"
    );
}
