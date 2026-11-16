use super::styled::{piped, shown};
use super::*;
use vpt_domain::identity::RecordingId;

fn latest(sandbox: &Sandbox, id: &RecordingId) -> ReleaseRecord {
    let ledger = ledger(sandbox);
    let sequence = ledger.releases(id).unwrap().into_iter().max().unwrap();
    ledger.release(sequence).unwrap().unwrap()
}

fn without_release(sandbox: &Sandbox, id: &RecordingId, text: &str) -> String {
    let release = latest(sandbox, id);
    let sequence = release.sequence;
    text.replace(release.destination.to_str().unwrap(), "<written>")
        .replace(release.report_path.to_str().unwrap(), "<report>")
        .replace(&format!("sequence\":{sequence}"), "sequence\":<n>")
        .replace(&format!("sequence: {sequence}\n"), "sequence: <n>\n")
}

fn with_a_candidate() -> (Sandbox, RecordingId) {
    let sandbox = fixture::configured("redact-terminal");
    let text = "Meet Alice with Bob via person@sample.test for 1234 and Carol";
    let id = recording::seed_segments(&sandbox, vec![recording::flag_word(7, "1234")], &[text]);
    json(&invoke(
        &sandbox,
        &["confirm", "term", "Alice", "-o", "json"],
    ));
    let bob = sandbox.path().join("home/.vpt/transcripts/Bob.md");
    fs::write(bob, "# Bob\n").unwrap();
    (sandbox, id)
}

#[test]
fn redact_at_a_terminal_marks_the_copy_its_report_and_what_it_masked() {
    let (sandbox, id) = with_a_candidate();
    let titles = [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine",
    ];
    let mut titles = titles.into_iter();
    let mut redact = |extra: &[&'static str]| {
        let title = titles.next().unwrap();
        [&["redact", id.as_str(), "--title", title], extra].concat()
    };
    let first = shown(&sandbox, &redact(&[]));
    let release = latest(&sandbox, &id);
    let (written, report) = (release.destination.display(), release.report_path.display());
    let masked = "  ✓ masked: email 1, note 1, term 1\n  ✓ flagged spans: 1 omitted, 0 marked\n";
    let unmasked = "unmasked: 1 candidate to read before sharing";
    let attention = format!("\nNeeds attention\n  ⚠ 1 warning to look at\n  1. {unmasked}\n");
    assert_eq!(
        first,
        format!(
            "  ✓ {id}: wrote {written}\n  ✓ private report: {report}\n{masked}  ⚠ {unmasked}\n    → line 8: Carol\n{attention}"
        )
    );
    let collision = "  ✓ used a timestamped name because the usual one was taken\n";
    assert!(shown(&sandbox, &redact(&[])).contains(collision));
    let plain = piped(&sandbox, &redact(&[]));
    assert!(plain.starts_with("Wrote "), "{plain}");
    assert!(plain.contains("\nCandidate, line 8: Carol\n"), "{plain}");
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        let terminal = shown(&sandbox, &redact(&[format]));
        let terminal = without_release(&sandbox, &id, &terminal);
        let pipe = piped(&sandbox, &redact(&[format]));
        assert_eq!(terminal, without_release(&sandbox, &id, &pipe), "{format}");
    }
}
