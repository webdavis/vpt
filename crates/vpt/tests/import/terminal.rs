use super::*;
use support::terminal::on_a_terminal;

#[test]
fn import_at_a_terminal_names_the_recording_it_ingested() {
    let sandbox = configured("");
    let source = file(&sandbox, "a.m4a", &m4a(1_787_604_456, 12, b"x"), None);
    let shown = on_a_terminal(
        sandbox.vpt().args(["import", path(&source)]),
        Duration::from_secs(3),
    );
    assert_eq!(shown.status.code(), Some(0), "{}", stderr(&shown));
    let again = json(&import(&sandbox, &[path(&source)]));
    let id = again["already_ingested"][0].as_str().unwrap();
    assert_eq!(stdout(&shown), format!("  ✓ {id}: ingested\n"));
}

#[test]
fn piped_import_prints_the_plain_lines_it_always_has() {
    let sandbox = configured("");
    let source = file(&sandbox, "a.m4a", &m4a(1_787_604_456, 12, b"x"), None);
    let text = run(sandbox.vpt().args(["import", path(&source)]));
    assert_eq!(text.status.code(), Some(0), "{}", stderr(&text));
    let again = json(&import(&sandbox, &[path(&source)]));
    let id = again["already_ingested"][0].as_str().unwrap();
    assert_eq!(
        stdout(&text),
        format!("Ingested 1 recording(s); deferred 0; skipped 0\n{id}\n")
    );
}
