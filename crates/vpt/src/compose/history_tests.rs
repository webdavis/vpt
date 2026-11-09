use super::history::commit;
use std::collections::BTreeSet;
use vpt_application::ports::*;
use vpt_domain::{
    history::{EventId, LedgerNamespace, RunId},
    occasion::OccasionId,
    run::{HistoryFact, HistoryRecord},
    time::UtcInstant,
};

struct Stop;
impl EventSink for Stop {
    fn after_commit(&mut self, receipt: &HistoryReceipt) -> SinkOutcome {
        SinkOutcome::Stop(Box::new(SinkStop {
            reason: SinkError::UnsafeState,
            committed: Some(receipt.clone()),
            warnings: BTreeSet::new(),
        }))
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        unreachable!()
    }
}

#[test]
fn brief_completion_survives_post_commit_failure_and_sink_stop() {
    let id = OccasionId::parse("2026-09-30T230000-cb4b7855ddbc").unwrap();
    let ns = LedgerNamespace::new([1; 16]);
    let run = RunId::new(ns, 1);
    for published in [false, true] {
        let receipt = HistoryReceipt {
            run,
            completed: vec![],
            records: vec![HistoryRecord {
                id: EventId::new(ns, 1),
                run,
                occurred_at: UtcInstant { secs: 10 },
                fact: if published {
                    HistoryFact::BriefPublished {
                        occasion: id.clone(),
                    }
                } else {
                    HistoryFact::BriefSaved {
                        occasion: id.clone(),
                    }
                },
                appended_at: None,
                export_position: None,
            }],
        };
        for write in [
            HistoryWrite::Committed(receipt.clone()),
            HistoryWrite::CommittedWithFailure {
                receipt: receipt.clone(),
                error: LedgerError::Busy,
            },
        ] {
            let error = commit(write, &mut Stop, &mut BTreeSet::new()).unwrap_err();
            let expected = if published {
                vec![id.to_string()]
            } else {
                vec![]
            };
            assert_eq!(error.completed, expected);
        }
    }
}
