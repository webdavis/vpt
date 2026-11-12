use super::*;
use serde_json::{Value, json};
use vpt_adapters::{RootDir, WriteLock};

pub(super) fn bulk_agent(sandbox: &Sandbox) {
    let dir = sandbox.path();
    let script = dir.join("bulk-agent.sh");
    fs::write(
        &script,
        "#!/bin/sh\n/bin/cat > /dev/null\nprintf '%s\\n' \"$2\" >> \"$1/calls\"\n\
         [ \"$2\" = \"$(/bin/cat \"$1/fail\" 2>/dev/null)\" ] && exit 1\n/bin/cat \"$1/proposal\"\n",
    )
    .unwrap();
    let known = dir.join("config/vpt/known-tags.txt");
    fixture::write_config(
        sandbox,
        &format!(
            "[synthesis]\ncommand = ['/bin/sh', {script:?}, {dir:?}, '{{id}}']\n\
             [tags]\nnew_tags = 'write'\nknown_tags_path = {known:?}\n"
        ),
    );
}

pub(super) fn propose(sandbox: &Sandbox, summary: &str, tags: &[&str]) {
    let tags: Vec<_> = tags
        .iter()
        .map(|tag| json!({"tag": tag, "reason": "private reason"}))
        .collect();
    let proposal = json!({
        "schema": "vpt.proposal/1", "by": "private agent",
        "summary": [{"text": summary, "sources": ["00:00-00:01"]}],
        "actions": [], "tags": tags, "relations": []
    });
    fs::write(sandbox.path().join("proposal"), proposal.to_string()).unwrap();
}

pub(super) fn bulk(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["synthesize"]).args(args).arg("--json"),
        Duration::from_secs(20),
    )
}

pub(super) fn calls(sandbox: &Sandbox) -> Vec<String> {
    fs::read_to_string(sandbox.path().join("calls"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

pub(super) fn ids(rows: &[RecordingRecord]) -> Vec<&str> {
    rows.iter().map(|row| row.id.as_str()).collect()
}

fn entry(result: &Value, index: usize) -> (&str, &Value, &Value, &Value) {
    let row = &result["recordings"][index];
    (
        row["id"].as_str().unwrap(),
        &row["summary_changed"],
        &row["tags_added"],
        &row["tags_removed"],
    )
}

pub(super) fn unaccepted(sandbox: &Sandbox, seed: u8) -> RecordingRecord {
    let row = fixture::record(sandbox, seed, StageStates::fresh());
    SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap())
        .unwrap()
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    row
}

#[test]
fn bulk_redo_reruns_every_accepted_recording_in_capture_order() {
    let (sandbox, rows) = note::configured_with(&[9, 7, 8]);
    unaccepted(&sandbox, 10);
    bulk_agent(&sandbox);
    let ordered = [rows[1].clone(), rows[2].clone(), rows[0].clone()];
    let order = ids(&ordered);
    propose(&sandbox, "first words", &["alpha", "beta"]);
    let result = note::success(&bulk(&sandbox, &["--all"]));
    assert_eq!(result["command"], "synthesize");
    assert_eq!(result["dry_run"], false);
    for (index, id) in order.iter().enumerate() {
        assert_eq!(
            entry(&result, index),
            (*id, &json!(true), &json!(2), &json!(0))
        );
        assert_eq!(result["recordings"][index]["status"], "succeeded");
    }
    assert_eq!(result["recordings"].as_array().unwrap().len(), 3);
    propose(&sandbox, "first words", &["alpha", "gamma"]);
    let output = bulk(&sandbox, &["--all"]);
    let result = note::success(&output);
    for (index, id) in order.iter().enumerate() {
        assert_eq!(
            entry(&result, index),
            (*id, &json!(false), &json!(1), &json!(1))
        );
    }
    assert_eq!(calls(&sandbox), [order.clone(), order].concat());
    for private in ["alpha", "beta", "gamma", "first words", "private"] {
        assert!(!stdout(&output).contains(private), "{private}");
    }
}

#[test]
fn bulk_redo_carries_on_past_a_failed_recording() {
    let (sandbox, rows) = note::configured_with(&[7, 8, 9]);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &["alpha"]);
    fs::write(sandbox.path().join("fail"), rows[1].id.as_str()).unwrap();
    let output = bulk(&sandbox, &["--all"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["kind"], "command");
    assert_eq!(error["error"]["ids"], json!([rows[1].id.as_str()]));
    assert_eq!(
        error["error"]["completed"],
        json!([rows[0].id.as_str(), rows[2].id.as_str()])
    );
    assert_eq!(
        error["error"]["diagnostics"],
        json!([format!(
            "{}: command: synthesis command exited unsuccessfully",
            rows[1].id
        )])
    );
    let ledger = ledger(&sandbox);
    assert_eq!(
        ledger.by_id(&rows[2].id).unwrap().unwrap().stages.synthesis,
        StageState::Succeeded
    );
    assert_eq!(calls(&sandbox), ids(&rows));
}

#[test]
fn since_keeps_recordings_captured_at_or_after_the_time() {
    let (sandbox, rows) = note::configured_with(&[7, 8, 9]);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &[]);
    let since = rows[1].captured_at.rfc3339();
    let result = note::success(&bulk(&sandbox, &["--all", "--since", &since]));
    let selected: Vec<_> = result["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert_eq!(selected, ids(&rows[1..]));
}

#[test]
fn a_dry_run_lists_the_recordings_without_starting_an_agent() {
    let (sandbox, rows) = note::configured_with(&[7, 8]);
    unaccepted(&sandbox, 10);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &[]);
    let before = fixture::snapshot(&sandbox);
    let result = note::success(&bulk(&sandbox, &["--all", "--dry-run"]));
    assert_eq!(result["dry_run"], true);
    assert_eq!(
        result["recordings"],
        json!(
            ids(&rows)
                .iter()
                .map(|id| json!({"id": id, "status": "planned"}))
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert!(calls(&sandbox).is_empty());
}

#[test]
fn bulk_redo_refuses_while_another_writer_holds_the_lock() {
    let (sandbox, _) = note::configured_with(&[7]);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &[]);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let output = bulk(&sandbox, &["--all"]);
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["message"], "another VPT writer is running");
    assert!(calls(&sandbox).is_empty());
}
