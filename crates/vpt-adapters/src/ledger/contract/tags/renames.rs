use super::seed;
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

fn rename(old: &str, new: &str, secs: i64) -> TagRename {
    TagRename {
        old: old.into(),
        new: new.into(),
        at: UtcInstant { secs },
    }
}

pub(crate) fn tag_renames_are_remembered_and_the_latest_rename_of_a_name_wins(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    assert!(ledger.tag_renames().unwrap().is_empty());
    let step = |value: TagRename| {
        ledger.commit_step(
            run,
            &WorkDelta {
                tag_rename: Some(value),
                ..Default::default()
            },
            &[(
                UtcInstant { secs: 50 },
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Completed,
                },
            )],
        )
    };
    assert!(matches!(
        step(rename("private-old", "Private_New", 50)),
        HistoryWrite::Committed(_)
    ));
    assert!(matches!(
        step(rename("other-old", "other-new", 55)),
        HistoryWrite::Committed(_)
    ));
    assert!(matches!(
        step(rename("private-old", "Private_Final", 60)),
        HistoryWrite::Committed(_)
    ));
    for (old, new) in [
        ("Private-Old", "elsewhere"),
        ("private old", "elsewhere"),
        ("private-old", "bad tag!"),
        ("private-old", "PRIVATE_OLD"),
    ] {
        assert!(matches!(
            step(rename(old, new, 70)),
            HistoryWrite::Unconfirmed(_)
        ));
    }
    assert_eq!(
        ledger.tag_renames().unwrap(),
        [
            rename("other-old", "other-new", 55),
            rename("private-old", "Private_Final", 60),
        ]
    );
    assert!(!format!("{:?}", ledger.tag_renames().unwrap()).contains("Private"));
}
