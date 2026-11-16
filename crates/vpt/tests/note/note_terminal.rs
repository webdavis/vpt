use super::styled::{document, piped, same_in_every_format, shown};
use super::*;

#[test]
fn note_write_at_a_terminal_names_the_note_it_wrote() {
    let (sandbox, row) = configured();
    let id = row.id.as_str();
    let args = ["note", "write", id];
    let first = shown(&sandbox, &args);
    let path = document(&sandbox, &args)["paths"][0].clone();
    let path = path.as_str().unwrap();
    assert_eq!(first, format!("  ✓ {id}: wrote {path}\n"));
    assert_eq!(piped(&sandbox, &args), format!("Wrote {path}\n"));
    same_in_every_format(&sandbox, &args);
    let verify = ["note", "verify", path, "--recording", id];
    assert!(shown(&sandbox, &verify).starts_with(&format!("  ✓ {path}: checked\n")));
}

#[test]
fn note_verify_at_a_terminal_says_when_a_note_has_no_findings() {
    let (sandbox, row) = configured();
    let path = sandbox.path().join("external.md");
    std::fs::write(&path, "- words [00:00-00:01]\n").unwrap();
    let path = path.to_str().unwrap();
    let args = ["note", "verify", path, "--recording", row.id.as_str()];
    assert_eq!(
        shown(&sandbox, &args),
        format!("  ✓ {path}: checked, report only\n  ✓ no findings\n")
    );
}

#[test]
fn note_verify_at_a_terminal_warns_once_per_finding_and_escapes_the_path() {
    let (sandbox, row) = configured();
    let path = sandbox.path().join("external\tnote.md");
    std::fs::write(
        &path,
        "- words [00:00-00:01]\n- Bob [00:00-00:01]\n- unsourced\n- words [00:30-00:31]\n",
    )
    .unwrap();
    let path = path.to_str().unwrap();
    let args = ["note", "verify", path, "--recording", row.id.as_str()];
    let escaped = path.escape_debug();
    assert_eq!(
        shown(&sandbox, &args),
        format!(
            "  ✓ {escaped}: checked, report only\n\
             \x20 ⚠ line 2: unsupported\n  ⚠ line 3: unsourced\n  ⚠ line 4: bad-reference\n\
             \n\
             Needs attention\n\
             \x20 ⚠ 3 warnings to look at\n\
             \x20 1. line 2: unsupported\n  2. line 3: unsourced\n  3. line 4: bad-reference\n"
        )
    );
    assert!(piped(&sandbox, &args).starts_with(&format!("3 findings in {path} (report only)\n")));
    same_in_every_format(&sandbox, &args);
}
