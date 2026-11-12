use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

fn configured(calendar_failure: bool) -> Sandbox {
    let sandbox = fixture::configured("brief-dam-context");
    let path = sandbox.path().join("bin/dam");
    let event = if calendar_failure {
        "printf '%s' '{\"error\":\"PRIVATE token\"}' >&2"
    } else {
        "printf '%s' '{\"events\":[]}'"
    };
    let task = r#"{"objects":[{"kind":"task","oid":"private-task","path":"work/team/","subject":"Private task <!-- vpt:brief start -->","body":"","labels":[],"depends":[],"reminders":[],"author":"owner","details":null,"task":{"done":false,"priority":2,"due":"2026-10-02"}}]}"#;
    fs::write(&path, format!("#!/bin/sh\n[ \"$3\" = --json ] && [ \"$4\" = --no-pull ] || exit 8\ncase \"$1 $2\" in\n'agenda path:calendar/work/') [ \"$5\" = --from ] && [ \"$7\" = --to ] && [ $# = 8 ] || exit 8; {event};;\n'ls kind:task & path:work/team/ & !done') printf '%s' '{task}';;\n*) exit 9;;\nesac\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    fixture::write_config(
        &sandbox,
        "[context]\ntype='dam'\ncalendars=['calendar/work/']\nprojects=['work/team/']\n",
    );
    sandbox
}

#[test]
fn dry_run_collects_scoped_context_without_writing_state_or_stores() {
    let sandbox = configured(false);
    let before = fixture::snapshot(&sandbox);
    let result = json(&invoke(&sandbox, &manual(&["--dry-run"])));
    assert_eq!(result["context"]["calendar"]["status"], "present");
    assert_eq!(result["context"]["tasks"]["status"], "present");
    assert_eq!(
        result["tasks"][0]["content"],
        "Private task <!-- vpt:brief start -->"
    );
    assert_eq!(result["tasks"][0]["source"], "dam");
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn manual_brief_survives_failed_calendar_and_saves_escaped_task_context() {
    let sandbox = configured(true);
    let output = invoke(&sandbox, &manual(&[]));
    let result = json(&output);
    assert_eq!(result["context"]["calendar"]["reason"], "failed");
    assert_eq!(result["context"]["tasks"]["status"], "present");
    assert!(!support::stdout(&output).contains("PRIVATE token"));
    assert!(!stderr(&output).contains("PRIVATE token"));
    let id = vpt_domain::occasion::OccasionId::parse(result["occasion"]["id"].as_str().unwrap())
        .unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let brief = ledger.brief(&id).unwrap().unwrap();
    assert_eq!(brief.snapshot.tasks.len(), 1);
    assert_eq!(brief.text.matches("<!-- vpt:brief start -->").count(), 1);
    assert!(brief.text.contains("Private task"));
    assert!(ledger.occasion(&id).unwrap().unwrap().briefed_at.is_some());
}

#[test]
fn manual_brief_commits_when_context_collection_crosses_a_clock_second() {
    let sandbox = configured(false);
    let path = sandbox.path().join("bin/dam");
    let script = fs::read_to_string(&path).unwrap();
    let event_output = "printf '%s' '{\"events\":[]}'";
    assert!(script.contains(event_output));
    fs::write(
        &path,
        script.replace(
            event_output,
            &format!("/bin/sleep 1 || exit 8; {event_output}"),
        ),
    )
    .unwrap();

    let result = json(&invoke(&sandbox, &manual(&[])));
    assert_eq!(result["context"]["calendar"]["status"], "present");
    assert_eq!(result["context"]["tasks"]["status"], "present");
    let id = vpt_domain::occasion::OccasionId::parse(result["occasion"]["id"].as_str().unwrap())
        .unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let brief = ledger.brief(&id).unwrap().unwrap();
    assert_eq!(fs::read_to_string(&brief.target).unwrap(), brief.text);
    assert_eq!(brief.snapshot.tasks.len(), 1);
    assert!(ledger.occasion(&id).unwrap().unwrap().briefed_at.is_some());
    assert!(ledger.pending_publications().unwrap().is_empty());
}

#[test]
fn a_dam_query_operator_in_a_selection_refuses_before_state_creation() {
    let sandbox = configured(false);
    let config = fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("calendar/work/", "calendar/work/ | kind:task");
    fs::write(sandbox.config_path(), config).unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &manual(&[]));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("context.calendars"));
    assert!(!stderr(&output).contains("kind:task"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn missing_google_credentials_are_named_without_creating_state_or_exposing_secrets() {
    let sandbox = fixture::configured("brief-google-credentials");
    for missing in ["client_id", "client_secret", "refresh_token"] {
        let mut extra =
            String::from("[context]\ntype='google'\ncalendars=['primary']\n[context.google]\n");
        for key in ["client_id", "client_secret", "refresh_token"] {
            extra.push_str(&format!(
                "{key}='{}'\n",
                if key == missing { "" } else { "PRIVATE-value" }
            ));
        }
        fixture::write_config(&sandbox, &extra);
        let before = fixture::snapshot(&sandbox);
        let output = invoke(&sandbox, &manual(&[]));
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains(&format!("context.google.{missing}")));
        assert!(!stderr(&output).contains("PRIVATE-value"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}
