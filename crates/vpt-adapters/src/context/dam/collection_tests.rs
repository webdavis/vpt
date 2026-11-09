use super::*;
use std::{fs, os::unix::fs::PermissionsExt};
use vpt_application::ports::ContextWindow;
use vpt_domain::{brief::ContextStatus, occasion::time::OccasionTime};

fn fixture(body: &str) -> (tempfile::TempDir, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("collector");
    fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let command = vec![
        script.to_string_lossy().into_owned(),
        dir.path().join("calls").to_string_lossy().into_owned(),
    ];
    (dir, command)
}
fn window() -> ContextWindow {
    ContextWindow {
        start: OccasionTime::parse("2026-10-01T23:00:00Z")
            .unwrap()
            .unix_second(),
        end: OccasionTime::parse("2026-10-02T01:00:00Z")
            .unwrap()
            .unix_second(),
    }
}
pub(super) fn request<'a>(calendars: &'a [String], projects: &'a [String]) -> ContextRequest<'a> {
    ContextRequest {
        window: Some(window()),
        tasks: true,
        calendars,
        projects,
    }
}

#[test]
fn collector_scopes_every_request_and_always_disables_pulls() {
    let (dir, command) =
        fixture("log=$1; shift\nprintf '%s\\n' \"$@\" >> \"$log\"\nprintf '%s' '{\"objects\":[]}'");
    let snapshot = DamContext { command: &command }
        .collect(&request(&["calendar/work/".into()], &["work/team/".into()]))
        .unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap();
    for day in ["2026-09-30", "2026-10-01", "2026-10-02", "2026-10-03"] {
        assert!(
            calls.contains(&format!(
                "ls\nkind:event & path:calendar/work/ & start:{day}\n--json\n--no-pull\n"
            )),
            "{calls}"
        );
    }
    assert!(calls.ends_with("ls\nkind:task & path:work/team/ & !done\n--json\n--no-pull\n"));
}

#[test]
fn missing_and_query_injected_selections_refuse_before_spawning() {
    let (dir, command) = fixture("echo invoked > \"$1\"");
    for (calendars, projects, key) in [
        (vec![], vec!["work/".into()], "context.calendars"),
        (vec!["calendar/".into()], vec![], "context.projects"),
        (
            vec!["calendar/ | kind:task".into()],
            vec!["work/".into()],
            "context.calendars",
        ),
    ] {
        assert_eq!(
            DamContext { command: &command }.collect(&request(&calendars, &projects)),
            Err(ContextConfigError { key }.into())
        );
    }
    assert!(!dir.path().join("calls").exists());
}

#[test]
fn failed_calendar_does_not_discard_successful_tasks_or_expose_provider_stderr() {
    let (_dir, command) = fixture(
        "case \"$3\" in kind:event*) echo '{\"error\":\"PRIVATE token\"}' >&2; printf '%s' '{\"objects\":[]}';; *) printf '%s' '{\"objects\":[]}' ;; esac",
    );
    let snapshot = DamContext { command: &command }
        .collect(&request(&["calendar/".into()], &["work/".into()]))
        .unwrap();
    assert!(matches!(
        snapshot.status.calendar,
        ContextStatus::Failed { .. }
    ));
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
    assert!(!format!("{snapshot:?}").contains("PRIVATE"));
}

#[test]
fn unrequested_halves_need_no_selection_and_do_not_spawn() {
    let (dir, command) = fixture("echo invoked > \"$1\"");
    let snapshot = DamContext { command: &command }
        .collect(&ContextRequest {
            window: None,
            tasks: false,
            calendars: &[],
            projects: &[],
        })
        .unwrap();
    assert_eq!(snapshot, ContextSnapshot::default());
    assert!(!dir.path().join("calls").exists());
}
