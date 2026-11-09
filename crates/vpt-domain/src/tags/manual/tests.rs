use super::*;

fn vocabulary() -> Vocabulary {
    Vocabulary::new(&["Project_Finance".into()]).unwrap()
}
fn now() -> UtcInstant {
    UtcInstant { secs: 100 }
}

#[test]
fn confirmation_reuses_vocabulary_spelling_and_normalizes_new_tags() {
    let known = vocabulary()
        .edit(&[], "PROJECT/FINANCE", Action::Confirm, 5, now())
        .unwrap();
    assert_eq!(known.tag.name, "Project_Finance");
    assert_eq!(known.tag.state, State::Confirmed);
    assert_eq!(known.tag.provenance, Provenance::Manual);
    assert_eq!(known.position, None);
    let new = vocabulary()
        .edit(&[], "New topic", Action::Confirm, 5, now())
        .unwrap();
    assert_eq!(new.tag.name, "new-topic");
    assert!(!format!("{new:?}").contains("new-topic"));
}

#[test]
fn promotion_rejection_and_retries_preserve_position_spelling_and_existing_confirmation_time() {
    let mut previous = vocabulary()
        .edit(&[], "Project_Finance", Action::Confirm, 5, now())
        .unwrap()
        .tag;
    previous.state = State::Suggested;
    previous.name = "PROJECT-Finance".into();
    previous.updated_at = None;
    let changed = vocabulary()
        .edit(&[previous], "project/finance", Action::Confirm, 5, now())
        .unwrap();
    assert_eq!(changed.position, Some(0));
    assert_eq!(changed.tag.name, "PROJECT-Finance");
    assert_eq!(changed.tag.updated_at, Some(now()));
    let repeated = vocabulary()
        .edit(
            std::slice::from_ref(&changed.tag),
            "project_finance",
            Action::Confirm,
            0,
            UtcInstant { secs: 200 },
        )
        .unwrap();
    assert_eq!(repeated, changed);
    let rejected = vocabulary()
        .edit(
            &[changed.tag],
            "project_finance",
            Action::Reject,
            0,
            UtcInstant { secs: 200 },
        )
        .unwrap();
    assert_eq!(rejected.tag.state, State::Rejected);
    assert_eq!(rejected.tag.updated_at, Some(UtcInstant { secs: 200 }));
}

#[test]
fn confirmation_limit_never_evicts_an_existing_tag_and_rejection_remains_available() {
    let first = vocabulary()
        .edit(&[], "first", Action::Confirm, 1, now())
        .unwrap()
        .tag;
    assert_eq!(
        vocabulary().edit(
            std::slice::from_ref(&first),
            "second",
            Action::Confirm,
            1,
            now()
        ),
        Err(ManualError::ConfirmedLimit)
    );
    assert!(
        vocabulary()
            .edit(&[first], "first", Action::Reject, 0, now())
            .is_ok()
    );
    assert_eq!(
        vocabulary().edit(&[], "first", Action::Confirm, 0, now()),
        Err(ManualError::ConfirmedLimit)
    );
}

#[test]
fn malformed_and_oversized_proposals_and_duplicate_saved_tags_are_refused() {
    for text in ["", "#tag", "123", "private!"] {
        assert_eq!(
            vocabulary().edit(&[], text, Action::Confirm, 5, now()),
            Err(ManualError::Syntax)
        );
    }
    assert_eq!(
        vocabulary().edit(&[], &"a".repeat(65_537), Action::Confirm, 5, now()),
        Err(ManualError::Capacity)
    );
    let tag = vocabulary()
        .edit(&[], "first", Action::Confirm, 5, now())
        .unwrap()
        .tag;
    assert_eq!(
        vocabulary().edit(&[tag.clone(), tag], "first", Action::Confirm, 5, now()),
        Err(ManualError::InvalidState)
    );
}
