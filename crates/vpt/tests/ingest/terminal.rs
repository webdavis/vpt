use super::*;
use std::time::Duration;
use support::terminal::on_a_terminal;
use vpt_application::ports::RecordingLedger;

fn shown(sandbox: &Sandbox, extra: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().arg("ingest").args(extra),
        Duration::from_secs(3),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &Sandbox, extra: &[&str]) -> String {
    let output = run(sandbox.vpt().arg("ingest").args(extra));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn only_id(sandbox: &Sandbox) -> String {
    let ledger = vpt_adapters::SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let rows = ledger.recordings().unwrap();
    assert_eq!(rows.len(), 1);
    rows[0].id.to_string()
}

#[test]
fn ingest_at_a_terminal_names_each_recording_it_ingested() {
    let sandbox = configured();
    add(&sandbox, "a.m4a");
    let shown = shown(&sandbox, &[]);
    assert_eq!(shown, format!("  ✓ {}: ingested\n", only_id(&sandbox)));
}

#[test]
fn piped_ingest_prints_the_plain_lines_it_always_has() {
    let sandbox = configured();
    add(&sandbox, "a.m4a");
    let piped = piped(&sandbox, &[]);
    assert_eq!(
        piped,
        format!(
            "Ingested 1 recording(s); deferred 0; skipped 0\n{}\n",
            only_id(&sandbox)
        )
    );
}

#[test]
fn a_dry_run_at_a_terminal_lists_each_file_it_would_ingest() {
    let sandbox = configured();
    let path = add(&sandbox, "a.m4a");
    assert_eq!(
        shown(&sandbox, &["--dry-run"]),
        format!("  · {}: would ingest\n", path.display())
    );
    assert_eq!(
        piped(&sandbox, &["--dry-run"]),
        format!(
            "Would ingest 1 recording(s); deferred 0; skipped 0\n{}\n",
            path.display()
        )
    );
    for format in ["--json", "--toon", "--yaml"] {
        assert_eq!(
            shown(&sandbox, &["--dry-run", format]),
            piped(&sandbox, &["--dry-run", format]),
            "{format}"
        );
    }
}

#[test]
fn ingest_with_nothing_new_says_so_in_one_line_at_a_terminal() {
    let sandbox = configured();
    assert_eq!(shown(&sandbox, &[]), "  · nothing new to ingest\n");
    assert_eq!(
        piped(&sandbox, &[]),
        "Ingested 0 recording(s); deferred 0; skipped 0\n"
    );
}
