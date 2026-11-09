mod boundaries;
pub(crate) use boundaries::*;
use vpt_application::ports::*;
use vpt_domain::{
    occasion::{OccasionDetails, OccasionKey, Participant, time::OccasionTime},
    run::{HistoryFact, RunKind},
    time::UtcInstant,
};

pub(crate) fn details() -> OccasionDetails {
    OccasionDetails {
        at: OccasionTime::parse("2026-09-30T23:00:00.123456789012-06:00").unwrap(),
        duration_secs: Some(1800),
        title: "Private meeting".into(),
        participants: vec![Participant {
            name: "Private name".into(),
            email: Some("private@example.test".into()),
        }],
        tags: vec!["Private tag".into()],
    }
}
pub(crate) fn saved(receipt: HistoryWrite<HistoryReceipt>) -> HistoryReceipt {
    match receipt {
        HistoryWrite::Committed(receipt) => receipt,
        other => panic!("{other:?}"),
    }
}
pub(crate) fn write(
    ledger: &impl RunHistory,
    run: vpt_domain::history::RunId,
    change: OccasionChange,
) -> HistoryWrite<HistoryReceipt> {
    let fact = (
        change.at,
        HistoryFact::OccasionSaved {
            occasion: change.occasion.id.clone(),
        },
    );
    ledger.commit_step(
        run,
        &WorkDelta {
            occasion: Some(change),
            ..Default::default()
        },
        &[fact],
    )
}

pub(crate) fn occasions_keep_identity_across_rescheduling_and_preserve_private_data(
    ledger: &(impl OccasionLedger + RunHistory),
) {
    let run = saved(ledger.begin_run(RunKind::Brief, UtcInstant { secs: 1 })).run;
    let key = OccasionKey::google("Private calendar", "Private occurrence").unwrap();
    let first =
        crate::ledger::occasion::prepare(ledger, key.clone(), details(), UtcInstant { secs: 2 })
            .unwrap();
    assert!(ledger.occasions().unwrap().is_empty());
    let receipt = saved(write(ledger, run, first.clone()));
    assert!(
        receipt.completed.is_empty(),
        "saving an occasion has not written a brief"
    );
    assert_eq!(
        ledger.occasion(&first.occasion.id).unwrap(),
        Some(first.occasion.clone())
    );
    assert_eq!(
        ledger.occasion_by_key(&key).unwrap(),
        Some(first.occasion.clone())
    );
    let mut rescheduled = details();
    rescheduled.title = "Private changed title".into();
    rescheduled.at = OccasionTime::parse("2026-10-04T08:00:00+02:00").unwrap();
    let next =
        crate::ledger::occasion::prepare(ledger, key, rescheduled.clone(), UtcInstant { secs: 3 })
            .unwrap();
    assert_eq!(next.occasion.id, first.occasion.id);
    saved(write(ledger, run, next.clone()));
    assert_eq!(ledger.occasions().unwrap(), [next.occasion]);
    assert!(!format!("{first:?} {:?}", ledger.pending_exports().unwrap()).contains("Private"));
    assert_eq!(
        ledger
            .occasion(&first.occasion.id)
            .unwrap()
            .unwrap()
            .details,
        rescheduled
    );
}

pub(crate) fn occasion_changes_reject_stale_identity_key_and_history(
    ledger: &(impl OccasionLedger + RunHistory),
) {
    let run = saved(ledger.begin_run(RunKind::Brief, UtcInstant { secs: 1 })).run;
    let first = crate::ledger::occasion::prepare(
        ledger,
        OccasionKey::dam("Private id").unwrap(),
        details(),
        UtcInstant { secs: 2 },
    )
    .unwrap();
    saved(write(ledger, run, first.clone()));
    let before = ledger.pending_exports().unwrap();
    assert!(matches!(
        write(ledger, run, first.clone()),
        HistoryWrite::Unconfirmed(_)
    ));
    for case in 0..5 {
        let mut change = first.clone();
        change.expected = Some(first.occasion.clone());
        let mut fact = (
            change.at,
            HistoryFact::OccasionSaved {
                occasion: first.occasion.id.clone(),
            },
        );
        match case {
            0 => {
                change.occasion.id =
                    vpt_domain::occasion::OccasionId::derive(&details().at, &super::digest(44))
            }
            1 => change.occasion.key = OccasionKey::dam("Other id").unwrap(),
            2 => fact.0.secs += 1,
            3 => change.occasion.briefed_at = Some(change.at),
            _ => change.occasion.brief_path = Some("/briefs/fabricated.md".into()),
        }
        let delta = WorkDelta {
            occasion: Some(change),
            ..Default::default()
        };
        assert!(
            matches!(
                ledger.commit_step(run, &delta, &[fact]),
                HistoryWrite::Unconfirmed(_)
            ),
            "case {case}"
        );
        assert_eq!(
            ledger.occasions().unwrap(),
            std::slice::from_ref(&first.occasion)
        );
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
}

pub(crate) fn occasion_save_rolls_back_when_later_history_fails(
    ledger: &(impl OccasionLedger + RunHistory),
) {
    let run = saved(ledger.begin_run(RunKind::Brief, UtcInstant { secs: 1 })).run;
    let change = crate::ledger::occasion::prepare(
        ledger,
        OccasionKey::dam("Private id").unwrap(),
        details(),
        UtcInstant { secs: 2 },
    )
    .unwrap();
    let facts = [
        (
            change.at,
            HistoryFact::OccasionSaved {
                occasion: change.occasion.id.clone(),
            },
        ),
        (
            change.at,
            HistoryFact::HookFailed {
                event: vpt_domain::history::EventId::new(run.namespace(), 999),
                failure: vpt_domain::run::HookFailureClass::Spawn,
            },
        ),
    ];
    assert!(matches!(
        ledger.commit_step(
            run,
            &WorkDelta {
                occasion: Some(change),
                ..Default::default()
            },
            &facts
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.occasions().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

macro_rules! cases {
    ($case:ident) => {
        $case!(occasions_keep_identity_across_rescheduling_and_preserve_private_data);
        $case!(occasion_changes_reject_stale_identity_key_and_history);
        $case!(manual_occasions_repeat_and_sort_by_exact_instant);
        $case!(invalid_occasion_data_is_refused_without_private_errors);
        $case!(occasion_save_rolls_back_when_later_history_fails);
    };
}
pub(crate) use cases;
