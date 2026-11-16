use super::styled::{document, piped, same_in_every_format, shown};
use super::tag_rename::{known_tags, proposed};
use super::*;
use vpt_domain::tags::State;

#[test]
fn tag_rename_at_a_terminal_marks_each_recording_and_what_it_left_alone() {
    let (sandbox, rows) = configured_with(&[7]);
    let id = rows[0].id.as_str();
    std::fs::write(known_tags(&sandbox), "old-topic\n").unwrap();
    super::tags::update(
        &sandbox,
        &rows[0].id,
        vec![proposed("old-topic", State::Confirmed)],
    );
    success(&invoke(&sandbox, id));
    let brief = sandbox.path().join("home/.vpt/briefs/Brief.md");
    std::fs::create_dir_all(brief.parent().unwrap()).unwrap();
    std::fs::write(&brief, "---\ntags: [old-topic]\n---\nMeeting brief\n").unwrap();
    let preview = ["tag", "rename", "old-topic", "new-topic", "--preview"];
    let note = document(&sandbox, &preview)["recordings"][0]["notes"][0].clone();
    let note = note.as_str().unwrap();
    let briefs =
        "  · briefs: 1 brief left alone, because a brief's tags are its meeting's labels\n";
    assert_eq!(
        shown(&sandbox, &preview),
        format!("  · {id}: would rename, {note}\n  · known tags: would rename\n{briefs}")
    );
    same_in_every_format(&sandbox, &preview);
    let rename = &preview[..4];
    assert_eq!(
        shown(&sandbox, rename),
        format!("  ✓ {id}: renamed, {note}\n  ✓ known tags: renamed\n{briefs}")
    );
    assert_eq!(
        shown(&sandbox, rename),
        format!("  · no recording or known tag carries that tag\n{briefs}")
    );
    assert!(
        piped(&sandbox, rename)
            .starts_with("No recording or known tag carries that tag; nothing changed\n")
    );
}
