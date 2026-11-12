use super::*;
use crate::{digest::Sha256Digest, tags::State, time::UtcInstant};

fn vocabulary() -> Vocabulary {
    Vocabulary::new(&["Project_Finance".into()]).unwrap()
}

fn tag(name: &str, state: State, manual: bool) -> Tag {
    Tag {
        name: name.into(),
        state,
        provenance: if manual {
            Provenance::Manual
        } else {
            Provenance::Proposal {
                by: "agent".into(),
                reason: "reason".into(),
                input: Sha256Digest([0; 32]),
            }
        },
        updated_at: Some(UtcInstant { secs: 7 }),
    }
}

#[test]
fn renaming_matches_any_spelling_of_the_old_name_and_keeps_state_and_position() {
    let rename = vocabulary().rename("Old Topic", "project/finance").unwrap();
    assert_eq!(rename.source(), "old-topic");
    assert_eq!(rename.target(), "Project_Finance");
    let before = [
        tag("first", State::Confirmed, false),
        tag("OLD_topic", State::Rejected, true),
        tag("last", State::Suggested, false),
    ];
    let after = rename.apply(&before).unwrap();
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1].name, "Project_Finance");
    assert_eq!(after[1].state, State::Rejected);
    assert_eq!(after[1].provenance, Provenance::Manual);
    assert_eq!(after[1].updated_at, before[1].updated_at);
    assert_eq!(after[2], before[2]);
    assert_eq!(rename.apply(&before[2..]), None);
}

#[test]
fn a_new_name_is_normalized_and_a_name_equal_after_normalization_is_refused() {
    let rename = vocabulary().rename("old", "Brand New").unwrap();
    assert_eq!(rename.target(), "brand-new");
    for (old, new) in [("", "new"), ("old", "#bad"), ("123", "new"), ("old", "")] {
        assert_eq!(vocabulary().rename(old, new), Err(RenameError::Syntax));
    }
    assert_eq!(
        vocabulary().rename("project finance", "PROJECT_FINANCE"),
        Err(RenameError::Same)
    );
}

#[test]
fn merging_keeps_one_entry_where_a_hand_set_state_beats_an_automatic_one() {
    let rename = vocabulary().rename("old", "new").unwrap();
    let merged = |old: Tag, new: Tag| {
        let after = rename
            .apply(&[new, tag("other", State::Confirmed, false), old])
            .unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].name, "new");
        assert_eq!(after[1].name, "other");
        (after[0].state, after[0].provenance == Provenance::Manual)
    };
    assert_eq!(
        merged(
            tag("old", State::Rejected, true),
            tag("new", State::Confirmed, false)
        ),
        (State::Rejected, true)
    );
    assert_eq!(
        merged(
            tag("old", State::Confirmed, false),
            tag("new", State::Rejected, true)
        ),
        (State::Rejected, true)
    );
    assert_eq!(
        merged(
            tag("old", State::Rejected, true),
            tag("new", State::Confirmed, true)
        ),
        (State::Confirmed, true)
    );
    assert_eq!(
        merged(
            tag("old", State::Confirmed, false),
            tag("new", State::Suggested, false)
        ),
        (State::Suggested, false)
    );
}

#[test]
fn a_merged_entry_keeps_the_earlier_position_and_the_recordings_spelling_of_the_new_name() {
    let rename = vocabulary().rename("old", "new").unwrap();
    let after = rename
        .apply(&[
            tag("old", State::Confirmed, true),
            tag("other", State::Confirmed, false),
            tag("NEW", State::Suggested, false),
        ])
        .unwrap();
    assert_eq!(
        after
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["NEW", "other"]
    );
    assert_eq!(after[0].state, State::Confirmed);
    assert!(!format!("{rename:?}").contains("old"));
}
