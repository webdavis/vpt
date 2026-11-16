use super::batch::{bulk_agent, propose};
use super::*;
use support::terminal::on_a_terminal;

fn shown(sandbox: &Sandbox, args: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().arg("analyze").args(args),
        Duration::from_secs(20),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &Sandbox, args: &[&str]) -> String {
    let output = support::run_with_budget(
        sandbox.vpt().arg("analyze").args(args),
        Duration::from_secs(20),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn notes(sandbox: &Sandbox, id: &vpt_domain::identity::RecordingId) -> [String; 2] {
    let ledger = ledger(sandbox);
    [NoteStage::Transcript, NoteStage::Analysis].map(|stage| {
        let note = ledger.note(id, stage).unwrap().unwrap();
        note.target.display().to_string()
    })
}

#[test]
fn analyze_at_a_terminal_names_its_notes_and_the_findings_left_to_verify() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let shown = shown(&sandbox, &[record.id.as_str()]);
    let id = &record.id;
    let [transcript, analysis] = notes(&sandbox, id);
    assert_eq!(
        shown,
        format!(
            "  ✓ {id}: analyzed\n  ✓ note: {transcript}\n  ✓ note: {analysis}\n\
             \x20 ⚠ {id}: 1 finding to verify\n    → vpt review {id}\n\n\
             Needs attention\n  ⚠ 1 warning to look at\n  1. {id}: 1 finding to verify\n"
        )
    );
}

#[test]
fn piped_analyze_prints_the_plain_line_it_always_has() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let id = &record.id;
    assert_eq!(
        piped(&sandbox, &[id.as_str()]),
        format!("Analyzed {id}; 1 findings\n")
    );
}

#[test]
fn a_dry_run_at_a_terminal_says_what_it_would_do_and_leaves_machine_output_alone() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let id = record.id.as_str();
    assert_eq!(
        shown(&sandbox, &[id, "--preview"]),
        format!("  · {id}: would analyze and publish its analysis note\n")
    );
    assert_eq!(
        piped(&sandbox, &[id, "--preview"]),
        format!("Would analyze {id} and publish its analysis note\n")
    );
    for format in ["--json", "--toon", "--yaml"] {
        assert_eq!(
            shown(&sandbox, &[id, "--preview", format]),
            piped(&sandbox, &[id, "--preview", format]),
            "{format}"
        );
    }
}

#[test]
fn analyze_all_at_a_terminal_gives_each_recording_one_row() {
    let (sandbox, rows) = note::configured_with(&[7, 8]);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &["alpha"]);
    let [one, two] = [&rows[0].id, &rows[1].id];
    assert_eq!(
        shown(&sandbox, &["--all", "--preview"]),
        format!("  · {one}: would analyze\n  · {two}: would analyze\n")
    );
    assert_eq!(
        piped(&sandbox, &["--all", "--preview"]),
        format!("Would analyze {one}\nWould analyze {two}\n")
    );
    assert_eq!(
        shown(&sandbox, &["--all"]),
        format!(
            "  ✓ {one}: summary changed, 1 tag added, 0 removed\n\
             \x20 ✓ {two}: summary changed, 1 tag added, 0 removed\n"
        )
    );
    assert_eq!(
        piped(&sandbox, &["--all"]),
        format!(
            "Analyzed {one}; summary unchanged; 0 tag(s) added, 0 removed\n\
             Analyzed {two}; summary unchanged; 0 tag(s) added, 0 removed\n"
        )
    );
}

#[test]
fn analyze_all_with_no_recordings_says_so_in_one_line_at_a_terminal() {
    let (sandbox, _) = note::configured_with(&[]);
    bulk_agent(&sandbox);
    assert_eq!(
        shown(&sandbox, &["--all"]),
        "  · no recordings to analyze\n"
    );
    assert_eq!(piped(&sandbox, &["--all"]), "no recordings\n");
}
