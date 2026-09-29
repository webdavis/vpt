use super::{snapshot, support::*};
use serde_json::{Value, json};
use vpt_application::ports::*;
use vpt_application::{RetentionCommit, RetentionError, reconcile_intents};
use vpt_domain::run::HistoryFact;
use vpt_domain::time::{UtcInstant, UtcOffset};

struct ObservedAt(UtcInstant);

impl Clock for ObservedAt {
    fn now(&self) -> UtcInstant {
        self.0
    }

    fn offset_at(&self, _: UtcInstant) -> UtcOffset {
        UtcOffset { secs: 0 }
    }
}

#[test]
fn missing_history_receipt_names_the_known_intent_at_detection_time() {
    let mut world = World::new();
    let record = world.record("legacy-gap.m4a", b"owned unmatched journal audio");
    let intent = new_intent(&record);
    let id = world.ledger.record_intent(&intent).unwrap();
    let pending = world.ledger.pending_intents().unwrap();
    let before = snapshot::tree(world.temp.path());
    let run = world.run();
    let failed = reconcile_intents(
        run,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths.audio,
        &mut world.sink,
    )
    .unwrap_err();
    assert_eq!(
        failed.error,
        RetentionError::MissingHistory { intent_id: id }
    );
    assert!(failed.progress.moved.is_empty());
    assert!(failed.progress.completed.is_empty());
    let Some(RetentionCommit::Step(receipt)) = failed.committed else {
        panic!("detected gap must retain its saved receipt");
    };
    assert_eq!(receipt.run, run);
    assert!(receipt.completed.is_empty());
    assert_eq!(receipt.records.len(), 1);
    let saved = &receipt.records[0];
    assert_eq!(saved.run, run);
    assert_eq!(saved.occurred_at, NOW);
    assert_ne!(saved.occurred_at, intent.recorded_at);
    let encoded: Value =
        serde_json::from_str(&vpt_protocol::encode_history(saved, NOW).unwrap()).unwrap();
    assert_eq!(encoded["schema"], "vpt.log/1");
    assert_eq!(
        encoded["fact"],
        json!({"tag": "missing_retention_history", "intent_id": id})
    );
    assert_eq!(
        *world.sink.trace.borrow(),
        vec![Action::Export(receipt.clone())]
    );
    let view = world.ledger.recovery_view().unwrap();
    assert_eq!(view.records.len(), 2);
    assert_eq!(view.records.last(), Some(saved));
    assert!(
        view.records
            .iter()
            .all(|record| !matches!(record.fact, HistoryFact::EffectIntent { .. }))
    );
    assert_eq!(world.ledger.pending_intents().unwrap(), pending);
    assert_eq!(snapshot::tree(world.temp.path()), before);
}

#[test]
fn repeated_missing_history_observations_keep_the_same_intent_and_current_times() {
    let mut world = World::new();
    let record = world.record("repeated-gap.m4a", b"owned repeated journal gap");
    let id = world.ledger.record_intent(&new_intent(&record)).unwrap();
    let pending = world.ledger.pending_intents().unwrap();
    let before = snapshot::tree(world.temp.path());
    let run = world.run();
    let times = [
        NOW,
        UtcInstant {
            secs: NOW.secs + 17,
        },
    ];
    let mut observed = Vec::new();
    for at in times {
        let failed = reconcile_intents(
            run,
            &world.ledger,
            &world.stores,
            &world.trash,
            &ObservedAt(at),
            &world.paths.audio,
            &mut world.sink,
        )
        .unwrap_err();
        assert_eq!(
            failed.error,
            RetentionError::MissingHistory { intent_id: id }
        );
        assert!(failed.progress.moved.is_empty());
        assert!(failed.progress.completed.is_empty());
        let Some(RetentionCommit::Step(receipt)) = failed.committed else {
            panic!("each detected gap must retain its saved receipt");
        };
        assert_eq!(receipt.run, run);
        assert!(receipt.completed.is_empty());
        assert_eq!(receipt.records.len(), 1);
        let saved = &receipt.records[0];
        assert_eq!(saved.run, run);
        assert_eq!(saved.occurred_at, at);
        let encoded: Value =
            serde_json::from_str(&vpt_protocol::encode_history(saved, at).unwrap()).unwrap();
        assert_eq!(
            encoded["fact"],
            json!({"tag": "missing_retention_history", "intent_id": id})
        );
        observed.push(saved.clone());
    }
    assert_ne!(observed[0].id, observed[1].id);
    let view = world.ledger.recovery_view().unwrap();
    assert_eq!(view.records.len(), 3);
    assert_eq!(&view.records[1..], observed.as_slice());
    assert!(
        view.records
            .iter()
            .all(|record| !matches!(record.fact, HistoryFact::EffectIntent { .. }))
    );
    assert_eq!(world.sink.trace.borrow().len(), 2);
    assert!(
        world
            .sink
            .trace
            .borrow()
            .iter()
            .all(|action| matches!(action, Action::Export(_)))
    );
    assert_eq!(world.ledger.pending_intents().unwrap(), pending);
    assert_eq!(snapshot::tree(world.temp.path()), before);
}
