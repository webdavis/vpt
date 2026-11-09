use super::*;
use crate::review::resolve::{ResolutionError, prepare};
use vpt_domain::review::ReviewAction;

#[test]
fn correction_changes_rendered_content_and_plans_a_term_without_mutating_accepted_words() {
    let mut snapshot = snapshot();
    let flag = flag(0..1, Class::ProperNoun);
    snapshot.flags.push(flag.clone());
    let accepted = snapshot.accepted.clone();
    let before = vpt_domain::note::transcript_content(
        &accepted.as_ref().unwrap().transcript,
        &snapshot.flags,
        false,
    )
    .unwrap();
    let action = ReviewAction::Correct("Private correction".into());
    let prepared = prepare(
        &mut snapshot,
        &id(),
        flag.identifier,
        &action,
        UtcInstant { secs: 10 },
    )
    .unwrap();
    assert_eq!(prepared.known_term.as_deref(), Some("Private correction"));
    let change = prepared.change.as_ref().unwrap();
    assert_eq!(change.expected, Resolution::Open);
    assert_eq!(change.resolution, snapshot.flags[0].resolution);
    assert_eq!(snapshot.accepted, accepted);
    let after = vpt_domain::note::transcript_content(
        &accepted.as_ref().unwrap().transcript,
        &snapshot.flags,
        false,
    )
    .unwrap();
    assert_ne!(before, after);
    assert!(after.contains("[corrected: Private correction]"));
    assert!(!format!("{prepared:?}").contains("Private correction"));
    let again = prepare(
        &mut snapshot,
        &id(),
        flag.identifier,
        &action,
        UtcInstant { secs: 20 },
    )
    .unwrap();
    assert!(again.change.is_none());
    assert_eq!(again.known_term, prepared.known_term);
    assert_eq!(
        snapshot.flags[0].resolution.at(),
        Some(UtcInstant { secs: 10 })
    );
}

#[test]
fn only_name_and_agreed_confirmations_learn_terms_and_dismissals_do_not() {
    for class in [
        Class::ProperNoun,
        Class::AgreedUnverified,
        Class::Numeric,
        Class::LowConfidence,
        Class::Other,
    ] {
        let mut snapshot = snapshot();
        let flag = flag(0..1, class);
        snapshot.flags.push(flag.clone());
        let prepared = prepare(
            &mut snapshot,
            &id(),
            flag.identifier,
            &ReviewAction::Confirm,
            UtcInstant { secs: 10 },
        )
        .unwrap();
        assert_eq!(
            prepared.known_term.is_some(),
            matches!(class, Class::ProperNoun | Class::AgreedUnverified)
        );
        let dismissed = prepare(
            &mut snapshot,
            &id(),
            flag.identifier,
            &ReviewAction::Dismiss,
            UtcInstant { secs: 20 },
        )
        .unwrap();
        assert!(dismissed.known_term.is_none());
        assert_eq!(
            snapshot.flags[0].resolution,
            Resolution::Dismissed {
                at: UtcInstant { secs: 20 }
            }
        );
    }
}

#[test]
fn rejected_actions_and_missing_flags_leave_the_snapshot_unchanged() {
    let mut snapshot = snapshot();
    let flag = flag(0..1, Class::ProperNoun);
    snapshot.flags.push(flag.clone());
    let before = snapshot.clone();
    assert!(matches!(
        prepare(
            &mut snapshot,
            &id(),
            flag.identifier,
            &ReviewAction::Correct("\0".into()),
            UtcInstant { secs: 2 }
        ),
        Err(ResolutionError::Action(_))
    ));
    assert_eq!(snapshot, before);
    assert!(matches!(
        prepare(
            &mut snapshot,
            &id(),
            Sha256Digest([99; 32]),
            &ReviewAction::Confirm,
            UtcInstant { secs: 2 }
        ),
        Err(ResolutionError::UnknownFlag)
    ));
    assert_eq!(snapshot, before);
}

#[test]
fn embedded_newlines_remain_correction_data_and_learn_one_whole_token_term() {
    let mut snapshot = snapshot();
    let flag = flag(0..1, Class::ProperNoun);
    snapshot.flags.push(flag.clone());
    let text = "Alice\n# operator heading";
    let prepared = prepare(
        &mut snapshot,
        &id(),
        flag.identifier,
        &ReviewAction::Correct(text.into()),
        UtcInstant { secs: 10 },
    )
    .unwrap();
    assert_eq!(
        prepared.known_term.as_deref(),
        Some("Alice # operator heading")
    );
    assert_eq!(
        snapshot.flags[0].resolution,
        Resolution::Corrected {
            text: text.into(),
            at: UtcInstant { secs: 10 }
        }
    );
    let rendered = vpt_domain::note::transcript_content(
        &snapshot.accepted.unwrap().transcript,
        &snapshot.flags,
        false,
    )
    .unwrap();
    assert!(rendered.contains("Alice&#10;&#35; operator heading"));
    assert!(!rendered.contains("\n# operator heading"));
}
