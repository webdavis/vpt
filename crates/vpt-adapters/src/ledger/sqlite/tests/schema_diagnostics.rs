use super::super::{SCHEMA_VERSION, SqliteLedger};
use crate::RootDir;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[test]
fn repository_trace_reports_ordered_phases_and_durations() {
    let (temp, _root) = super::root();
    let started = Instant::now();
    let child = super::child_for_root(
        "ledger_contract_schema_trace",
        temp.path(),
        std::process::id(),
        started,
    )
    .unwrap();
    let output = child.finish().expect("bounded ledger trace child");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(
        output.status.success(),
        "owned trace child failed: {}",
        output.status
    );
    let stderr = String::from_utf8(output.stderr).expect("text-only phase markers");
    let phases: Vec<_> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("VPT_PHASE "))
        .collect();
    let mut next = 0;
    let mut previous_elapsed_ms = 0;
    let mut selected = Vec::new();
    for expected in [
        "root.open.start",
        "root.open.end",
        "ledger.open.start",
        "ledger.open.end",
        "repo.read.start",
        "db.open.start",
        "db.open.end",
        "db.operation.start",
        "db.operation.end",
        "db.close.start",
        "db.close.end",
        "repo.read.end",
    ] {
        let (index, line) = phases
            .iter()
            .enumerate()
            .skip(next)
            .find(|(_, line)| line.split_ascii_whitespace().next() == Some(expected))
            .unwrap_or_else(|| panic!("missing ordered phase {expected}: {phases:?}"));
        let elapsed_ms = numeric_marker(line, "elapsed_ms=");
        assert!(
            elapsed_ms >= previous_elapsed_ms,
            "phase clocks moved backward: {line}"
        );
        selected.push((*line, elapsed_ms));
        previous_elapsed_ms = elapsed_ms;
        next = index + 1;
    }
    for (start, end) in [(0, 1), (2, 3), (4, 11), (5, 6), (7, 8), (9, 10)] {
        let duration_ms = numeric_marker(selected[end].0, "duration_ms=");
        let elapsed_delta_ms = selected[end].1 - selected[start].1;
        assert!(
            duration_ms <= elapsed_delta_ms && elapsed_delta_ms - duration_ms <= 1,
            "duration differs from its phase interval: {:?}",
            selected[end]
        );
    }
    for (line, _) in selected {
        eprintln!("VPT_TRACE {line}");
    }
}

fn numeric_marker(line: &str, name: &str) -> u128 {
    line.split_ascii_whitespace()
        .find_map(|part| part.strip_prefix(name))
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("phase lacks numeric {name}: {line}"))
}

pub(super) fn run(root: &RootDir) {
    let ledger = SqliteLedger::open_with_timeout(root, Duration::from_millis(50)).unwrap();
    assert_eq!(ledger.schema_version().unwrap(), SCHEMA_VERSION);
}

static ORIGIN: OnceLock<Option<Instant>> = OnceLock::new();

pub(crate) fn start(phase: &'static str) -> Option<Instant> {
    let origin = ORIGIN.get_or_init(|| {
        std::env::var("VPT_DATABASE_SCENARIO")
            .is_ok_and(|scenario| {
                scenario.starts_with("ledger_contract_") || scenario == "ledger_schema_foreign_keys"
            })
            .then(Instant::now)
    });
    origin.map(|origin| {
        let started = Instant::now();
        eprintln!(
            "VPT_PHASE {phase} elapsed_ms={}",
            started.duration_since(origin).as_millis()
        );
        started
    })
}

pub(crate) fn end(started: Option<Instant>, phase: &'static str) {
    if let Some(started) = started {
        let origin = ORIGIN.get().copied().flatten().expect("started trace");
        let completed = Instant::now();
        eprintln!(
            "VPT_PHASE {phase} elapsed_ms={} duration_ms={}",
            completed.duration_since(origin).as_millis(),
            completed.duration_since(started).as_millis()
        );
    }
}

#[test]
fn phase_forwarding_selects_orders_bounds_and_escapes_owned_markers() {
    let mut stderr = b"unrelated VPT_PHASE hidden\nVPT_PHASEX ignored\nVPT_PHASE first elapsed_ms=1\nVPT_PHASE second\t\x1b\\\xff".to_vec();
    stderr.extend_from_slice(&[b'x'; 100]);
    stderr.extend_from_slice(b"\nunrelated hostile \x1b\xff\nVPT_PHASE final elapsed_ms=9");
    assert_eq!(
        formatted_phases(&stderr),
        [
            "first elapsed_ms=1".to_owned(),
            format!("second\\t\\x1b\\\\\\xff{}", "x".repeat(86)),
            "final elapsed_ms=9".to_owned(),
        ]
    );
}

pub(super) fn formatted_phases(stderr: &[u8]) -> Vec<String> {
    stderr
        .split(|byte| *byte == b'\n')
        .filter_map(|line| line.strip_prefix(b"VPT_PHASE "))
        .map(|phase| phase[..phase.len().min(96)].escape_ascii().to_string())
        .collect()
}
