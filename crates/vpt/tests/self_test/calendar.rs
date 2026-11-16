use crate::fixture::{Trial, script};
use crate::support::{stderr, stdout};
use serde_json::Value;
use std::path::PathBuf;
use vpt_adapters::SystemClock;
use vpt_application::ports::Clock;

struct Logs {
    _directory: tempfile::TempDir,
    arguments: PathBuf,
}

fn calendar(trial: &Trial, events: usize, body: &str) -> Logs {
    let directory = tempfile::tempdir().unwrap();
    let logs = Logs {
        arguments: directory.path().join("arguments"),
        _directory: directory,
    };
    let now = SystemClock.now().secs;
    let agenda = trial.path().join("agenda.json");
    let rows = (0..events)
        .map(|index| {
            crate::support::dam::event(&format!("made-up-{index}"), "Made-up", now, now + 1)
        })
        .collect();
    std::fs::write(&agenda, crate::support::dam::agenda(rows)).unwrap();
    let program = script(
        &trial.sandbox,
        "made-up-calendar",
        &format!(
            "printf '%s\\n' \"$@\" >> {:?}\n{body}\n{}",
            logs.arguments,
            crate::support::dam::script(&agenda)
        ),
    );
    trial.configure(&format!(
        "[context]\ntype = 'dam'\ncalendars = ['calendar/work/']\n[context.dam]\ncommand = [{program:?}]\n"
    ));
    logs
}

fn row(document: &Value) -> &Value {
    let checks = document["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1, "{document}");
    assert_eq!(checks[0]["name"], "calendar");
    let rows = checks[0]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{document}");
    assert_eq!(rows[0]["name"], "events");
    &rows[0]
}

#[test]
fn the_events_of_today_are_counted_and_nothing_is_saved() {
    let trial = Trial::new("test-calendar-events");
    let logs = calendar(&trial, 2, "");
    let before = crate::observations_fixture::snapshot(&trial.sandbox);
    let document = trial.json(&["test", "calendar"], 0);
    let events = row(&document);
    assert_eq!(events["result"], "ok", "{document}");
    let detail = events["detail"].as_str().unwrap();
    assert!(
        detail.starts_with("2 events today from `made-up-calendar` in "),
        "{detail}"
    );
    assert!(detail.contains("up to that program"), "{detail}");
    let arguments = std::fs::read_to_string(&logs.arguments).unwrap();
    let lines: Vec<&str> = arguments.lines().collect();
    assert_eq!(lines[0], "agenda", "{arguments}");
    let from = lines.iter().position(|line| *line == "--from").unwrap();
    let to = lines.iter().position(|line| *line == "--to").unwrap();
    assert!(lines[from + 1] < lines[to + 1]);
    assert!(!lines.contains(&"ls"), "{arguments}");
    assert_eq!(
        crate::observations_fixture::snapshot(&trial.sandbox),
        before
    );
}

#[test]
fn a_failing_calendar_program_is_an_issue_with_advice() {
    let trial = Trial::new("test-calendar-fails");
    calendar(&trial, 0, "exit 1");
    let document = trial.json(&["test", "calendar"], 1);
    let events = row(&document);
    assert_eq!(events["result"], "issue");
    assert!(
        events["advice"]
            .as_str()
            .unwrap()
            .contains("vpt doctor calendar")
    );
}

#[test]
fn without_a_source_a_named_calendar_test_is_refused_naming_the_key() {
    let trial = Trial::new("test-calendar-unset");
    let output = trial.output(&["test", "calendar"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("`context.type`"),
        "{}",
        stderr(&output)
    );
}
