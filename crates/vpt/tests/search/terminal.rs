use super::*;
use support::terminal::on_a_terminal;

fn shown(sandbox: &Sandbox, args: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().arg("search").args(args),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &Sandbox, args: &[&str]) -> String {
    let mut command = vec!["search"];
    command.extend(args);
    let output = invoke(sandbox, &command);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

#[test]
fn search_at_a_terminal_marks_each_hit_with_its_moment_and_quote() {
    let (sandbox, older, _) = corpus("search-terminal");
    assert_eq!(
        shown(&sandbox, &["friday"]),
        format!(
            "  ✓ {older}@04:10: The budget REVIEW moves to Friday (title-1, transcript)\n\
             \x20 ✓ {older}@04:10: The budget review moves to Friday (title-1, analysis)\n"
        )
    );
    assert_eq!(shown(&sandbox, &["absent"]), "  · no matches\n");
}

#[test]
fn a_cut_search_at_a_terminal_says_how_to_see_more() {
    let (sandbox, _, newer) = corpus("search-terminal-limit");
    assert_eq!(
        shown(&sandbox, &["budget", "--limit", "1"]),
        format!(
            "  ✓ {newer}@00:05: Review the budget before lunch (title-2, transcript)\n\
             \x20 ⚠ showed the first 1 match\n\
             \x20   → see more with --limit\n\
             \n\
             Needs attention\n\
             \x20 ⚠ 1 warning to look at\n\
             \x20 1. showed the first 1 match\n"
        )
    );
}

#[test]
fn piped_search_and_every_format_keep_their_bytes() {
    let (sandbox, older, _) = corpus("search-terminal-piped");
    assert_eq!(
        piped(&sandbox, &["friday"]),
        format!(
            "{older}@04:10  title-1  transcript  The budget REVIEW moves to Friday\n\
             {older}@04:10  title-1  analysis  The budget review moves to Friday\n"
        )
    );
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        let args = [format, "friday"];
        assert_eq!(shown(&sandbox, &args), piped(&sandbox, &args), "{format}");
    }
}
