use super::*;
use crate::MemoryLedger;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use vpt_application::ports::{HistoryWrite, WorkDelta};
use vpt_domain::history::{Event, EventCounts, EventDetail, EventKind, EventPath, EventState};
use vpt_domain::run::{HistoryFact, RunKind};

mod failures;
mod live;
mod optional;
mod partial;
mod recovery;
mod replay;

struct Fixture {
    temp: TempDir,
    state: RootDir,
    history: MemoryLedger,
}

impl Fixture {
    fn new() -> Self {
        let temp = crate::test_temp::tempdir().expect("owned event fixture");
        let path = temp.path().canonicalize().expect("canonical fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private fixture");
        Self {
            temp,
            state: RootDir::open(&path).expect("retained state"),
            history: MemoryLedger::new(),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.temp.path().join(name)
    }

    fn log_path(&self) -> PathBuf {
        self.path("events.jsonl")
    }

    fn saved_event(&self, kind: EventKind, state: EventState) -> HistoryReceipt {
        let event = Event {
            kind,
            state,
            occasion: None,
            recording: None,
            detail: EventDetail::None,
            counts: EventCounts::default(),
            paths: vec![],
            occurred_at: at(11),
        };
        self.save_event(event)
    }

    fn save_event(&self, event: Event) -> HistoryReceipt {
        let started = committed(self.history.begin_run(RunKind::Ingest, at(10)));
        committed(self.history.commit_step(
            started.run,
            &WorkDelta::default(),
            &[(at(11), HistoryFact::Event(event))],
        ))
    }

    fn documents(&self) -> Vec<Value> {
        assert!(self.log_path().exists(), "saved records were not exported");
        let bytes = fs::read(self.log_path()).expect("owned event log");
        assert!(bytes.ends_with(b"\n"));
        bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).expect("one complete JSON line"))
            .collect()
    }
}

fn at(secs: i64) -> UtcInstant {
    UtcInstant { secs }
}

fn committed<T: std::fmt::Debug>(write: HistoryWrite<T>) -> T {
    match write {
        HistoryWrite::Committed(value) => value,
        other => panic!("fixture history write: {other:?}"),
    }
}

fn write_hook(path: &Path) {
    fs::write(
        path,
        b"#!/bin/bash\nset -euo pipefail\ncapture=$1\nlog=$2\nmode=$3\nshift 3\nprintf x >> \"${capture}.calls\"\nprintf '%s\\0' \"$@\" > \"${capture}.argv\"\ncat > \"${capture}.stdin\"\nif [[ -f \"$log\" ]]; then cat \"$log\" > \"${capture}.seenlog\"; fi\nprintf 'child-private-sentinel' >&2\ncase \"$mode\" in\n  exit7) exit 7 ;;\n  signal) kill -TERM $$ ;;\n  hang) /bin/sleep 30 ;;\n  *) exit 0 ;;\nesac\n",
    )
    .expect("owned hook fixture");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("executable fixture");
}

fn command(script: &Path, capture: &Path, log: &Path, mode: &str, extra: &[&str]) -> Vec<String> {
    let mut argv = vec![
        script.to_str().unwrap().to_owned(),
        capture.to_str().unwrap().to_owned(),
        log.to_str().unwrap().to_owned(),
        mode.to_owned(),
    ];
    argv.extend(extra.iter().map(|value| (*value).to_owned()));
    argv
}

fn sink<'a>(
    fixture: &'a Fixture,
    report: &'a [String],
    alert: &'a [String],
) -> LoggedEvents<'a, MemoryLedger, impl FnMut() -> UtcInstant> {
    LoggedEvents::new(&fixture.state, &fixture.history, report, alert, || at(20))
}
