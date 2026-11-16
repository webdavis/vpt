use super::*;
use std::time::Duration;
use support::terminal::on_a_terminal;

fn shown(sandbox: &Sandbox, id: &str, extra: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().args(["review", "list", id]).args(extra),
        Duration::from_secs(3),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &Sandbox, id: &str, extra: &[&str]) -> String {
    let output = run(sandbox.vpt().args(["review", "list", id]).args(extra));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

#[test]
fn review_list_at_a_terminal_marks_each_open_flag_as_a_warning() {
    let (sandbox, id) = configured();
    let result = json(&run(sandbox.vpt().args([
        "review",
        "list",
        id.as_str(),
        "-o",
        "json",
    ])));
    let hex = |list: &str, index: usize| result[list][index]["identifier"].as_str().unwrap();
    let numeric = format!(
        "00:02 numeric: \"42\" | \"43\", 1 occurrence, {}",
        hex("flags", 0)
    );
    let mumble = format!(
        "00:07 low-confidence: \"um\", 1 occurrence, {}",
        hex("flags", 1)
    );
    let name = format!(
        "00:04 agreed-unverified: \"SecretAlice\", 2 occurrences, {}",
        hex("flags", 2)
    );
    let diagnostic = hex("diagnostics", 0);
    assert_eq!(
        shown(&sandbox, id.as_str(), &[]),
        format!(
            "  ⚠ {numeric}\n  ⚠ {mumble}\n  ⚠ {name}\n  · diagnostic: single-engine {diagnostic}\n\
             \n\
             Needs attention\n\
             \x20 ⚠ 3 warnings to look at\n\
             \x20 1. {numeric}\n  2. {mumble}\n  3. {name}\n"
        )
    );
}

fn untranscribed(name: &str) -> (Sandbox, String) {
    let sandbox = fixture::configured(name);
    let row = fixture::record(&sandbox, 8, StageStates::fresh());
    fixture::seed(&sandbox, vec![row.clone()]);
    (sandbox, row.id.as_str().to_owned())
}

#[test]
fn an_untranscribed_review_says_so_in_one_line() {
    let (sandbox, id) = untranscribed("review-terminal-empty");
    assert_eq!(shown(&sandbox, &id, &[]), "  · no accepted transcript\n");
}

#[test]
fn piped_review_list_prints_the_plain_lines_it_always_has() {
    let (sandbox, id) = configured();
    let plain = piped(&sandbox, id.as_str(), &[]);
    assert!(plain.starts_with(&format!(
        "Review {id}: 3 open lexical flags, 1 diagnostics\n[00:02] numeric (1 occurrences) "
    )));
}

#[test]
fn review_list_formats_are_the_same_bytes_at_a_terminal_and_through_a_pipe() {
    let (sandbox, id) = untranscribed("review-terminal-formats");
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        assert_eq!(
            shown(&sandbox, &id, &[format]),
            piped(&sandbox, &id, &[format]),
            "{format}"
        );
    }
}
