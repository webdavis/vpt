use super::*;
use std::time::Duration;
use support::terminal::on_a_terminal;
use vpt_application::ports::NoteLedger;

fn shown(sandbox: &Sandbox, id: &str, extra: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().args(["transcribe", id]).args(extra),
        Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &Sandbox, id: &str, extra: &[&str]) -> String {
    let output = support::run_with_budget(
        sandbox.vpt().args(["transcribe", id]).args(extra),
        Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn note(sandbox: &Sandbox, record: &vpt_application::ports::RecordingRecord) -> String {
    ledger(sandbox)
        .note(&record.id, vpt_application::ports::NoteStage::Transcript)
        .unwrap()
        .unwrap()
        .target
        .display()
        .to_string()
}

#[test]
fn transcribe_at_a_terminal_names_the_engine_the_note_and_what_needs_attention() {
    let (sandbox, record) = configured();
    let shown = shown(&sandbox, record.id.as_str(), &[]);
    let id = &record.id;
    assert_eq!(
        shown,
        format!(
            "  ✓ {id}: transcribed with custom (fr-CA)\n  ✓ note: {}\n\
             \x20 ⚠ {id}: single-engine\n\n\
             Needs attention\n  ⚠ 1 warning to look at\n  1. {id}: single-engine\n",
            note(&sandbox, &record)
        )
    );
}

#[test]
fn piped_transcribe_prints_the_plain_lines_it_always_has() {
    let (sandbox, record) = configured();
    let piped = piped(&sandbox, record.id.as_str(), &[]);
    assert_eq!(
        piped,
        format!(
            "Transcribed {} using custom (fr-CA)\nWrote {}\n",
            record.id,
            note(&sandbox, &record)
        )
    );
}

#[test]
fn a_dry_run_at_a_terminal_says_what_it_would_do_and_leaves_machine_output_alone() {
    let (sandbox, record) = configured();
    let id = record.id.as_str();
    assert_eq!(
        shown(&sandbox, id, &["--preview"]),
        format!("  · {id}: would transcribe in fr-CA with custom\n")
    );
    assert_eq!(
        piped(&sandbox, id, &["--preview"]),
        format!("Would transcribe {id} in fr-CA using custom\n")
    );
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        assert_eq!(
            shown(&sandbox, id, &["--preview", format]),
            piped(&sandbox, id, &["--preview", format]),
            "{format}"
        );
    }
}
