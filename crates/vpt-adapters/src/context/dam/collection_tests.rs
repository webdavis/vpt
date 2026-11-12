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
fn collector_reads_the_selected_agenda_window_then_shows_only_selected_events() {
    let (dir, command) = fixture(concat!(
        "log=$1; shift\nprintf '%s\\n' \"$@\" >> \"$log\"\n",
        "case \"$1\" in\n",
        "agenda) printf '%s' '{\"events\":[",
        "{\"oid\":\"inside\",\"subject\":\"s\",\"path\":\"calendar/work/\",\"labels\":[],\"status\":\"confirmed\",\"transparency\":\"busy\",\"all_day\":false,\"start\":\"2026-10-02T00:00:00Z\",\"end\":\"2026-10-02T00:30:00Z\",\"busy\":true},",
        "{\"oid\":\"before\",\"subject\":\"s\",\"path\":\"calendar/work/\",\"labels\":[],\"status\":\"confirmed\",\"transparency\":\"busy\",\"all_day\":false,\"start\":\"2026-10-01T22:00:00Z\",\"end\":\"2026-10-01T23:30:00Z\",\"busy\":true}",
        "]}';;\n",
        "show) printf '%s' '{\"oid\":\"inside\",\"kind\":\"event\",\"subject\":\"s\",\"path\":\"calendar/work/\",\"event\":{\"attendees\":[{\"email\":\"a@example.org\",\"response\":\"accepted\"}]}}';;\n",
        "*) printf '%s' '{\"objects\":[]}';;\n",
        "esac",
    ));
    let snapshot = DamContext { command: &command }
        .collect(&request(
            &["calendar/work/".into(), "calendar/home/".into()],
            &["work/team/".into()],
        ))
        .unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
    assert_eq!(snapshot.occasions.len(), 1);
    assert_eq!(
        snapshot.occasions[0].details.participants[0]
            .email
            .as_deref(),
        Some("a@example.org")
    );
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap();
    assert_eq!(
        calls,
        concat!(
            "agenda\npath:calendar/work/ | path:calendar/home/\n--json\n--no-pull\n",
            "--from\n2026-10-01T23:00:00Z\n--to\n2026-10-02T01:00:00Z\n",
            "show\ninside\n--json\n--no-pull\n",
            "ls\nkind:task & path:work/team/ & !done\n--json\n--no-pull\n",
        )
    );
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
        "case \"$2\" in agenda) echo '{\"error\":\"PRIVATE token\"}' >&2; printf '%s' '{\"events\":[]}';; *) printf '%s' '{\"objects\":[]}' ;; esac",
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
