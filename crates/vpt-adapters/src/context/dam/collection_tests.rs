use super::*;
use std::{fs, os::unix::fs::PermissionsExt};
use vpt_application::ports::ContextWindow;
use vpt_domain::{brief::ContextStatus, occasion::time::OccasionTime};

pub(super) fn fixture(body: &str) -> (tempfile::TempDir, Vec<String>) {
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
    let snapshot = DamContext {
        command: &command,
        max_age: &[],
    }
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
            DamContext {
                command: &command,
                max_age: &[]
            }
            .collect(&request(&calendars, &projects)),
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
    let snapshot = DamContext {
        command: &command,
        max_age: &[],
    }
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
    let snapshot = DamContext {
        command: &command,
        max_age: &[],
    }
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

#[test]
fn max_age_is_passed_to_agenda_and_a_stale_refusal_fails_only_the_calendar() {
    let (dir, command) = fixture(concat!(
        "log=$1; shift\nprintf '%s\\n' \"$@\" >> \"$log\"\n",
        "case \"$1\" in\n",
        "agenda) printf '%s' '{\"error\":{\"kind\":\"refused\",\"rule\":\"stale_remote\",\"message\":\"PRIVATE\",\"oids\":[]}}' >&2; exit 4;;\n",
        "*) printf '%s' '{\"objects\":[]}';;\n",
        "esac",
    ));
    let max_age = ["gcal=15m".into(), "work=1h".into()];
    let snapshot = DamContext {
        command: &command,
        max_age: &max_age,
    }
    .collect(&request(&["calendar/".into()], &["work/".into()]))
    .unwrap();
    assert_eq!(
        snapshot.status.calendar,
        ContextStatus::Failed {
            detail: Failure::Stale.detail().into()
        }
    );
    assert!(Failure::Stale.detail().contains("stale"));
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
    assert!(!format!("{snapshot:?}").contains("PRIVATE"));
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap();
    assert!(
        calls.starts_with("agenda\npath:calendar/\n--json\n--no-pull\n--from\n2026-10-01T23:00:00Z\n--to\n2026-10-02T01:00:00Z\n--max-age\ngcal=15m\n--max-age\nwork=1h\nls\n"),
        "{calls}"
    );
}

#[test]
fn an_unknown_max_age_remote_fails_the_calendar_naming_the_setting() {
    let (_dir, command) = fixture(concat!(
        "case \"$2\" in\n",
        "agenda) printf '%s' '{\"error\":{\"kind\":\"refused\",\"rule\":\"no_such_remote\",\"message\":\"PRIVATE\",\"oids\":[]}}' >&2; exit 4;;\n",
        "*) printf '%s' '{\"objects\":[]}';;\n",
        "esac",
    ));
    let max_age = ["missing=15m".into()];
    let snapshot = DamContext {
        command: &command,
        max_age: &max_age,
    }
    .collect(&request(&["calendar/".into()], &["work/".into()]))
    .unwrap();
    let detail = snapshot.status.calendar.detail().unwrap();
    assert_eq!(
        detail,
        "context.dam.max_age names a remote that dam does not know or that its token may not read; use a name from dam remote list"
    );
    assert!(detail.contains("context.dam.max_age") && !detail.contains("stale"));
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
}

#[test]
fn malformed_max_age_refuses_before_spawning() {
    let (dir, command) = fixture("echo invoked > \"$1\"");
    for value in ["gcal", "=15m", "gcal=", "-x=1h", "gcal=15 m"] {
        let max_age = [value.into()];
        assert_eq!(
            DamContext {
                command: &command,
                max_age: &max_age,
            }
            .collect(&request(&["calendar/".into()], &["work/".into()])),
            Err(ContextConfigError {
                key: "context.dam.max_age"
            }
            .into())
        );
    }
    assert!(!dir.path().join("calls").exists());
}

#[test]
fn a_missing_dam_or_one_without_agenda_fails_the_calendar_naming_what_is_missing() {
    let (dir, mut command) =
        fixture("printf \"error: unrecognized subcommand 'agenda'\" >&2; exit 2");
    let rejected = DamContext {
        command: &command,
        max_age: &[],
    }
    .collect(&request(&["calendar/".into()], &["work/".into()]))
    .unwrap();
    command[0] = dir.path().join("absent").to_string_lossy().into_owned();
    let missing = DamContext {
        command: &command,
        max_age: &[],
    }
    .collect(&request(&["calendar/".into()], &["work/".into()]))
    .unwrap();
    for (snapshot, words) in [
        (rejected, ["subcommand or flag", "newer dam"]),
        (missing, ["dam", "context.dam.command"]),
    ] {
        let detail = snapshot.status.calendar.detail().unwrap().to_owned();
        assert!(words.iter().all(|word| detail.contains(word)), "{detail}");
    }
}
