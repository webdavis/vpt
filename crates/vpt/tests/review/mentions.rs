use super::{resolution::*, *};
use std::fs;
use vpt_domain::relations::Kind;

fn indexed(sandbox: &Sandbox, name: &str) {
    let root = sandbox.path().join("home/.vpt/briefs");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(format!("{name}.md")), "External note").unwrap();
}

#[test]
fn a_new_confirmation_derives_all_mentions_in_the_same_note_update() {
    let (sandbox, id, terms) = setup();
    indexed(&sandbox, "SecretAlice");
    assert!(!terms.exists());
    let before = inputs(&sandbox, &id).review.accepted;
    let flag = identifier(&sandbox, &id, "agreed-unverified");
    json(&resolve(&sandbox, &id, &flag, &["--confirm"]));
    let after = inputs(&sandbox, &id);
    assert_eq!(after.review.accepted, before);
    assert_eq!(after.relations.len(), 1);
    assert_eq!(after.relations[0].relation.kind, Kind::Mentions);
    assert_eq!(after.relations[0].relation.ranges, [4..5, 6..7]);
    assert!(after.notes[0].text.contains("- Mentions: [SecretAlice]"));
    assert_eq!(fs::read_to_string(&terms).unwrap(), "SecretAlice\n");
}

#[test]
fn correction_text_does_not_invent_mentions_in_the_accepted_word_stream() {
    let (sandbox, id, _) = setup();
    indexed(&sandbox, "Private correction");
    let flag = identifier(&sandbox, &id, "numeric");
    json(&resolve(
        &sandbox,
        &id,
        &flag,
        &["--correct", "Private correction"],
    ));
    let after = inputs(&sandbox, &id);
    assert!(after.relations.is_empty());
    assert!(
        after.notes[0]
            .text
            .contains("[corrected: Private correction]")
    );
}
