use super::*;

const ACTIONS: &str = r#"[
    {"text":"- send the secret invoice","sources":["00:00-00:01"],"owner":"me","due":{"date":"2026-11-19","time":"15:00","sources":["00:00-00:01"]}},
    {"text":"Dana reviews the secret draft","sources":["00:00-00:01"],"owner":{"other":"Dana"},"due":{"date":"2026-11-20","sources":[]}},
    {"text":"someone books the secret room","sources":[],"owner":{"other":null}},
    {"text":"maybe follow up secretly","sources":["00:00-00:01"],"due":{"date":"2026-11-21","sources":["00:00-00:01"]}}
]"#;

pub(super) fn propose(sandbox: &Sandbox, actions: &str) {
    let proposal = format!(
        r#"{{"schema":"vpt.proposal/1","by":"private agent","summary":[],"actions":{actions},"tags":[],"relations":[]}}"#
    );
    fs::write(sandbox.path().join("captured.proposal"), proposal).unwrap();
}

pub(super) fn spoken() -> (Sandbox, RecordingRecord) {
    let (sandbox, mut rows) = note::configured_saying(&[7], "um private words tomorrow");
    (sandbox, rows.remove(0))
}

pub(super) fn configure(sandbox: &Sandbox) -> std::path::PathBuf {
    configure_with(sandbox, "")
}

pub(super) fn configure_with(sandbox: &Sandbox, extra: &str) -> std::path::PathBuf {
    let script = sandbox.path().join("todo-tool.sh");
    let log = sandbox.path().join("sent");
    fs::write(
        &script,
        "#!/bin/sh\nlog=\"$1\"\nshift\nprintf '%s\\037' \"$@\" >> \"$log.attempts\"\nprintf '\\n' >> \"$log.attempts\"\n[ -e \"$log.fail\" ] && exit \"$(/bin/cat \"$log.fail\")\"\nprintf '%s\\037' \"$@\" >> \"$log\"\nprintf '\\n' >> \"$log\"\n",
    )
    .unwrap();
    agent(
        sandbox,
        &format!(
            "[actions]\ncommand = ['/bin/sh', {script:?}, {log:?}, 'add', '--id', '{{id}}', '--moment', '{{moment}}', '--owner', '{{owner}}', '--name', '{{owner_name}}', '--due', '{{due}}', '--date', '{{due_date}}', '--time', '{{due_time}}', '--recording', '{{recording}}', '--note', '{{note}}', '--link', '{{link}}', '--', '{{text}}']\n{extra}"
        ),
    );
    propose(sandbox, ACTIONS);
    log
}

pub(super) fn sent(log: &std::path::Path) -> Vec<Vec<String>> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|line| {
            line.trim_end_matches('\u{1f}')
                .split('\u{1f}')
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

pub(super) fn note_path(sandbox: &Sandbox, id: &vpt_domain::identity::RecordingId) -> String {
    let note = ledger(sandbox)
        .note_path(id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    note.to_str().unwrap().to_owned()
}

fn link(note: &str) -> String {
    let encoded: String = note
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    format!("file://{encoded}")
}

fn args(
    id: &str,
    owner: &[&str],
    due: &[&str],
    (action_id, moment): (&str, &str),
    tail: &[String],
) -> Vec<String> {
    ["add", "--id", action_id, "--moment", moment, "--owner"]
        .iter()
        .chain(owner)
        .chain(due)
        .map(|field| (*field).to_owned())
        .chain(["--recording".into(), id.into()])
        .chain(tail.iter().cloned())
        .collect()
}

fn text(sent: &[String]) -> &str {
    sent.last().unwrap()
}

#[test]
fn each_action_reaches_the_command_once_with_its_id_owner_due_date_and_note() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    let output = invoke(&sandbox, record.id.as_str(), false);
    let result = note::success(&output);
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 4, "failed": 0, "refused": 0})
    );
    let id = record.id.as_str();
    let note = note_path(&sandbox, &record.id);
    let tail = |text: &str| {
        ["--note", &note, "--link", &link(&note), "--", text]
            .map(str::to_owned)
            .to_vec()
    };
    let at = format!("{id}@00:00");
    assert_eq!(
        sent(&log),
        [
            args(
                id,
                &["me"],
                &[
                    "--due",
                    "2026-11-19T15:00",
                    "--date",
                    "2026-11-19",
                    "--time",
                    "15:00"
                ],
                (&at, &at),
                &tail("- send the secret invoice"),
            ),
            args(
                id,
                &["other", "--name", "Dana"],
                &[],
                (&format!("{at}#2"), &at),
                &tail("Dana reviews the secret draft"),
            ),
            args(
                id,
                &["other"],
                &[],
                (&format!("{id}#1"), id),
                &tail("someone books the secret room")
            ),
            args(
                id,
                &["unknown"],
                &["--due", "2026-11-21", "--date", "2026-11-21"],
                (&format!("{at}#3"), &at),
                &tail("maybe follow up secretly"),
            ),
        ]
    );
    assert!(!stdout(&output).contains("secret"));
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("secret"));
}

#[test]
fn a_redo_resends_a_reworded_action_under_its_id_and_nothing_else() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let unchanged = note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(
        unchanged["actions"],
        serde_json::json!({"sent": 0, "failed": 0, "refused": 0})
    );
    assert_eq!(sent(&log).len(), 4);
    propose(
        &sandbox,
        &ACTIONS.replace(
            "Dana reviews the secret draft",
            "Dana checks the secret draft",
        ),
    );
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let sent = sent(&log);
    assert_eq!(sent.len(), 5);
    assert_eq!(sent[4][2], sent[1][2]);
    assert_eq!(text(&sent[4]), "Dana checks the secret draft");
}

#[test]
fn an_action_a_redo_drops_is_not_sent_again_or_withdrawn() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let kept: Vec<serde_json::Value> = serde_json::from_str(ACTIONS).unwrap();
    propose(&sandbox, &serde_json::json!(kept[..2]).to_string());
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(sent(&log).len(), 4);
}

#[test]
fn a_failed_send_keeps_the_analysis_and_is_retried_by_the_next_analysis() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    fs::write(log.with_extension("fail"), "1").unwrap();
    let result = note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(
        result["warnings"],
        serde_json::json!(["configured action command failed"])
    );
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 0, "failed": 4, "refused": 0})
    );
    assert!(
        ledger(&sandbox)
            .note(&record.id, NoteStage::Analysis)
            .unwrap()
            .is_some()
    );
    assert!(sent(&log).is_empty());
    fs::remove_file(log.with_extension("fail")).unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(sent(&log).len(), 4);
}

#[test]
fn an_action_refused_with_a_listed_exit_code_is_never_sent_again() {
    let (sandbox, record) = spoken();
    let log = configure_with(&sandbox, "final_exit_codes = [4]\n");
    fs::write(log.with_extension("fail"), "4").unwrap();
    let result = note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 0, "failed": 0, "refused": 4})
    );
    assert_eq!(
        result["warnings"],
        serde_json::json!(["configured action command refused an action"])
    );
    fs::remove_file(log.with_extension("fail")).unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(sent(&log.with_extension("attempts")).len(), 4);
}

#[test]
fn a_dry_run_sends_nothing() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    note::success(&invoke(&sandbox, record.id.as_str(), true));
    assert!(sent(&log.with_extension("attempts")).is_empty());
}

#[test]
fn without_an_actions_command_analysis_reports_no_action_counts() {
    let (sandbox, record) = spoken();
    agent(&sandbox, "");
    propose(&sandbox, ACTIONS);
    let result = note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert!(result.get("actions").is_none());
}

#[test]
fn identical_twin_actions_are_each_sent_under_their_own_id() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    let twin = r#"{"text":"call the secret bank","sources":["00:00-00:01"],"owner":"me"}"#;
    propose(&sandbox, &format!("[{twin},{twin}]"));
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let ids: Vec<_> = sent(&log).into_iter().map(|args| args[2].clone()).collect();
    let at = format!("{}@00:00", record.id.as_str());
    assert_eq!(ids, [at.clone(), format!("{at}#2")]);
}
