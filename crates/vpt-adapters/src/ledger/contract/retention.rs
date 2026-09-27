use super::{digest, record};
use std::path::PathBuf;
use vpt_application::ports::{ArtifactKind, NewIntent, RetentionIntent, RetentionJournal};
use vpt_domain::time::UtcInstant;

pub(crate) fn fixture(seed: u8) -> NewIntent {
    NewIntent {
        kind: ArtifactKind::Audio,
        recording: record(seed).id,
        path: PathBuf::from(format!("/store/audio/{seed}.m4a")),
        expected: digest(seed.wrapping_add(80)),
        recorded_at: UtcInstant {
            secs: 5_000 + i64::from(seed),
        },
    }
}

fn recorded(id: i64, new: NewIntent) -> RetentionIntent {
    RetentionIntent {
        id,
        kind: new.kind,
        recording: new.recording,
        path: new.path,
        expected: new.expected,
        recorded_at: new.recorded_at,
    }
}

pub(crate) fn a_recorded_intent_is_pending_with_its_fields<J: RetentionJournal>(journal: &J) {
    let new = fixture(1);
    let id = journal.record_intent(&new).expect("recorded");
    assert!(id > 0);
    assert_eq!(
        journal.pending_intents().expect("pending"),
        vec![recorded(id, new)]
    );
}

pub(crate) fn completing_an_intent_removes_it_and_keeps_the_others_in_order<J: RetentionJournal>(
    journal: &J,
) {
    let first = fixture(2);
    let second = fixture(3);
    let third = fixture(4);
    let first_id = journal.record_intent(&first).expect("first");
    let second_id = journal.record_intent(&second).expect("second");
    let third_id = journal.record_intent(&third).expect("third");
    assert!(first_id < second_id && second_id < third_id);
    journal
        .complete_intent(first_id, UtcInstant { secs: 6_000 })
        .expect("completed");
    assert_eq!(
        journal.pending_intents().expect("pending"),
        vec![recorded(second_id, second), recorded(third_id, third)]
    );
}

pub(crate) fn repeated_and_unknown_completion_change_no_pending_intents<J: RetentionJournal>(
    journal: &J,
) {
    journal
        .complete_intent(99_999, UtcInstant { secs: 7_000 })
        .expect("unknown");
    assert!(journal.pending_intents().expect("empty").is_empty());
    let new = fixture(5);
    let id = journal.record_intent(&new).expect("recorded");
    assert_eq!(id, 1, "unknown completion created an intent");
    journal
        .complete_intent(id, UtcInstant { secs: 7_001 })
        .expect("completed");
    journal
        .complete_intent(id, UtcInstant { secs: 7_002 })
        .expect("repeated");
    assert!(
        journal
            .pending_intents()
            .expect("empty after completion")
            .is_empty()
    );
}
