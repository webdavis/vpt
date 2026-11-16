use super::*;
use std::time::Duration;
use support::terminal::on_a_terminal;

fn shown(sandbox: &Sandbox, extra: &[&str]) -> String {
    let output = on_a_terminal(sandbox.vpt().arg("run").args(extra), Duration::from_secs(8));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &Sandbox, extra: &[&str]) -> String {
    let output =
        support::run_with_budget(sandbox.vpt().arg("run").args(extra), Duration::from_secs(8));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn ids(sandbox: &Sandbox) -> Vec<String> {
    let mut ids: Vec<_> = ledger(sandbox)
        .recordings()
        .unwrap()
        .into_iter()
        .map(|row| row.id.to_string())
        .collect();
    ids.sort();
    ids
}

#[test]
fn a_run_at_a_terminal_names_each_step_and_closes_with_what_needs_attention() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    add(&sandbox, "two.m4a", 1_787_604_476);
    let shown = shown(&sandbox, &[]);
    let [one, two] = &ids(&sandbox)[..] else {
        panic!("two recordings")
    };
    assert_eq!(
        shown,
        format!(
            "  ✓ ingested: 2 recordings\n  ✓ transcribed: 2 recordings\n  ✓ notes: 2 published\n\
             \x20 ⚠ {one}: single-engine\n  ⚠ {two}: single-engine\n\n\
             Needs attention\n  ⚠ 2 warnings to look at\n\
             \x20 1. {one}: single-engine\n  2. {two}: single-engine\n"
        )
    );
}

#[test]
fn a_piped_run_prints_the_plain_summary_it_always_has() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    let piped = piped(&sandbox, &[]);
    let id = &ids(&sandbox)[0];
    assert_eq!(
        piped,
        format!(
            "Run complete: 1 ingested, 1 transcribed, 1 notes, 0 analyzed, 0 released\n\
             Warning: {id}: single-engine\nMoved 0 file(s); kept 0\n"
        )
    );
}

#[test]
fn a_dry_run_at_a_terminal_lists_what_it_would_do_and_leaves_machine_output_alone() {
    let sandbox = configured();
    let source = add(&sandbox, "one.m4a", 1_787_604_456);
    assert_eq!(
        shown(&sandbox, &["--dry-run"]),
        format!("  · {}: would ingest\n", source.display())
    );
    assert_eq!(
        piped(&sandbox, &["--dry-run"]),
        format!(
            "Run plan: 1 ingested, 0 transcribed, 0 notes, 0 analyzed, 0 released\n\
             Would ingest {}\nWould move 0 file(s); kept 0\n",
            source.display()
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
fn a_run_with_nothing_to_do_says_so_in_one_line_at_a_terminal() {
    let sandbox = configured();
    assert_eq!(shown(&sandbox, &[]), "  · nothing to do\n");
    assert_eq!(
        piped(&sandbox, &[]),
        "Run complete: 0 ingested, 0 transcribed, 0 notes, 0 analyzed, 0 released\n\
         Moved 0 file(s); kept 0\n"
    );
}
