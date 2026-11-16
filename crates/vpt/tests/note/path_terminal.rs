use super::*;
use std::time::Duration;
use support::terminal::on_a_terminal;

fn path(sandbox: &Sandbox, id: &str, format: Option<&str>) -> std::process::Command {
    let mut command = sandbox.vpt();
    command
        .args(["path", id, "--stage", "transcript"])
        .args(format);
    command
}

#[test]
fn path_at_a_terminal_prints_the_bare_path_so_it_can_be_pasted() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let note = saved(&sandbox, &row);
    let id = row.id.as_str();
    let budget = Duration::from_secs(5);
    for format in [
        None,
        Some("--output=json"),
        Some("--output=toon"),
        Some("--output=yaml"),
    ] {
        let piped = support::run_with_budget(&mut path(&sandbox, id, format), budget);
        let shown = on_a_terminal(&mut path(&sandbox, id, format), budget);
        assert_eq!(shown.status.code(), Some(0), "{}", stderr(&shown));
        assert_eq!(stdout(&shown), stdout(&piped), "{format:?}");
    }
    let shown = on_a_terminal(&mut path(&sandbox, id, None), budget);
    assert_eq!(stdout(&shown), format!("{}\n", note.target.display()));
}
