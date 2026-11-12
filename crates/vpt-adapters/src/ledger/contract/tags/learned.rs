use super::seed;
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(crate) fn learned_tags_are_remembered_until_forgotten(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    assert!(ledger.learned_tags().unwrap().is_empty());
    let step = |learned: &[&str], forgotten: &[&str]| {
        ledger.commit_step(
            run,
            &WorkDelta {
                learned_tags: Some(LearnedTagChange {
                    learned: learned.iter().map(|key| (*key).into()).collect(),
                    forgotten: forgotten.iter().map(|key| (*key).into()).collect(),
                }),
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
        step(&["private-beta", "alpha"], &[]),
        HistoryWrite::Committed(_)
    ));
    assert!(matches!(
        step(&["gamma"], &["private-beta"]),
        HistoryWrite::Committed(_)
    ));
    for invalid in ["Private_Beta", "two words", ""] {
        assert!(matches!(
            step(&[invalid], &[]),
            HistoryWrite::Unconfirmed(_)
        ));
    }
    assert_eq!(ledger.learned_tags().unwrap(), ["alpha", "gamma"]);
}
