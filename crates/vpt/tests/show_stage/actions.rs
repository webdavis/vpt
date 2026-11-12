use super::*;
use serde_json::Value;
use vpt_domain::identity::RecordingId;

fn analyze(name: &str, actions: Value) -> (Sandbox, RecordingId, std::process::Output) {
    let sandbox = fixture::configured(name);
    let id = recording::seed_flags(&sandbox, false);
    let script = sandbox.path().join("agent.sh");
    let proposal = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    let document = json!({"schema": "vpt.proposal/1", "by": "fixture", "summary": [{"text": "meet Alice", "sources": ["00:00-00:01"]}], "actions": actions, "tags": [], "relations": []});
    fs::write(&proposal, document.to_string()).unwrap();
    fixture::write_config(
        &sandbox,
        &format!("[analysis]\ncommand=['/bin/sh', {script:?}, {proposal:?}]\n"),
    );
    let output = invoke(&sandbox, &["analyze", id.as_str(), "--json"]);
    (sandbox, id, output)
}

fn analyzed(name: &str, actions: Value) -> (Sandbox, RecordingId, Value, String) {
    let (sandbox, id, output) = analyze(name, actions);
    json(&output);
    let shown = json(&show(&sandbox, id.as_str(), "analysis"));
    let note = analysis_note(&sandbox, &id).text;
    (sandbox, id, shown, actions_section(&note))
}

fn analysis_note(sandbox: &Sandbox, id: &RecordingId) -> NoteArtifact {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .note(id, NoteStage::Analysis)
        .unwrap()
        .unwrap()
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
    let (_sandbox, id, shown, section) = analyzed(
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
    let (_sandbox, _id, shown, section) = analyzed(
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
    let (_sandbox, _id, shown, section) = analyzed(
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

#[test]
fn a_spoken_due_date_reaches_show_and_the_note_and_survives_a_rewrite() {
    let (sandbox, id, shown, section) = analyzed(
        "show-action-due",
        json!([
            action(
                "email Bob",
                json!({"owner": "me", "due": {"date": "2026-11-19", "sources": ["00:00-00:01"]}})
            ),
            action(
                "call Alice",
                json!({"due": {"date": "2026-11-20", "time": "15:00", "sources": ["00:05-00:09"]}})
            ),
        ]),
    );
    assert_eq!(
        shown["actions"][0]["due"],
        json!({"date": "2026-11-19", "time": null, "sources": [{"start": 0, "end": 1, "range": "00:00-00:01"}], "finding": null})
    );
    assert_eq!(
        shown["actions"][1]["due"],
        json!({"date": "2026-11-20", "time": "15:00", "sources": [{"start": 5, "end": 9, "range": "00:05-00:09"}], "finding": null})
    );
    for line in shown["actions"].as_array().unwrap() {
        assert_eq!(line["moment"], format!("{}@00:00", id.as_str()));
        assert_eq!(line["finding"], json!(null));
    }
    let expected = "## Actions\n\n- email Bob [00:00-00:01] [owner:: me] [due:: 2026-11-19]\n- call Alice [00:00-00:01] [due:: 2026-11-20 15:00]\n";
    assert_eq!(section, expected);
    json(&invoke(&sandbox, &["note", "write", id.as_str(), "--json"]));
    let note = analysis_note(&sandbox, &id);
    assert_eq!(actions_section(&note.text), expected);
    let verified = json(&invoke(
        &sandbox,
        &[
            "verify-note",
            note.target.to_str().unwrap(),
            "--recording",
            id.as_str(),
            "--json",
        ],
    ));
    assert_eq!(verified["flags"], json!([]));
    assert_eq!(
        json(&show(&sandbox, id.as_str(), "analysis"))["actions"],
        shown["actions"]
    );
}

#[test]
fn a_due_date_without_a_valid_source_is_withheld_and_flagged_while_the_action_stays() {
    let (_sandbox, _id, shown, section) = analyzed(
        "show-action-due-withheld",
        json!([
            action(
                "send notes",
                json!({"due": {"date": "2026-11-19", "sources": []}})
            ),
            action("send notes", json!({"due": {"date": "2026-11-19"}})),
            action(
                "send notes",
                json!({"due": {"date": "2026-11-19", "sources": ["00:30-00:40"]}})
            ),
            action(
                "send notes",
                json!({"due": {"date": "2026-11-19", "time": "09:00", "sources": ["Thursday"]}})
            ),
        ]),
    );
    let findings = ["unsourced", "unsourced", "bad-reference", "bad-reference"];
    for (line, finding) in shown["actions"].as_array().unwrap().iter().zip(findings) {
        assert_eq!(line["text"], "send notes");
        assert_eq!(line["finding"], json!(null));
        assert_eq!(
            line["due"],
            json!({"date": null, "time": null, "sources": [], "finding": finding})
        );
    }
    assert_eq!(
        section,
        format!("## Actions\n\n{}", "- send notes [00:00-00:01]\n".repeat(4))
    );
}

#[test]
fn a_malformed_due_date_refuses_the_proposal() {
    for date in ["2026-02-30", "19 November", "2026-11-19T15:00"] {
        let due = json!({"due": {"date": date, "sources": ["00:00-00:01"]}});
        let (_sandbox, _id, output) = analyze(
            "show-action-due-malformed",
            json!([action("send notes", due)]),
        );
        assert_ne!(output.status.code(), Some(0), "{date}");
        assert!(
            stderr(&output).contains("analysis output document is invalid"),
            "{date}"
        );
    }
}
