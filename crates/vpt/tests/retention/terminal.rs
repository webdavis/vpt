use super::styled::{document, piped, same_in_every_format, shown};
use super::*;

#[test]
fn retention_at_a_terminal_marks_each_file_it_moves() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let row = expired_audio(&sandbox);
    let preview = ["retention", "run", "--preview"];
    let moved = document(&sandbox, &preview)["moved"][0]["path"].clone();
    let path = moved.as_str().unwrap();
    assert_eq!(
        shown(&sandbox, &preview),
        format!("  · audio: would move {path}\n")
    );
    assert!(piped(&sandbox, &preview).starts_with("Would move 1 file(s); kept 0\n"));
    same_in_every_format(&sandbox, &preview);
    let id = row.id.as_str();
    assert_eq!(
        shown(&sandbox, &preview[..2]),
        format!("  ✓ audio: moved {path}\n  ✓ completed: {id}\n")
    );
    assert_eq!(shown(&sandbox, &preview[..2]), "  · nothing to move\n");
}
