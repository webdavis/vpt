use super::super::{decode, encode};
use serde_json::json;
use vpt_domain::history::{EventId, LedgerNamespace};
use vpt_domain::run::{HistoryFact, RecoveryBaseline, RecoveryEvidence};

#[test]
fn recovery_progress_baseline_round_trips_exact_frontiers() {
    let namespace = LedgerNamespace::new([0xab; 16]);
    let baseline = RecoveryBaseline {
        generation: Some(4),
        pending_from: Some(EventId::new(namespace, 2)),
        through: Some(EventId::new(namespace, 7)),
    };
    let fact = HistoryFact::RecoveryProgress {
        evidence: RecoveryEvidence {
            affected_runs: vec![],
            first_failure_observed_at: None,
            logging_restored_observed_at: None,
            failures: 0,
            recovered_records: 0,
            completed_work: 0,
            failed_work: 0,
            unknown_work: 0,
            deferred_work: 0,
            completed_effects: 0,
            failed_effects: 0,
            unresolved_effects: 0,
            missing_history: 0,
        },
        baseline,
    };
    let stored = encode(&fact).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(
        value["baseline"],
        json!({
            "generation": 4,
            "pending_from": baseline.pending_from.unwrap().render(),
            "through": baseline.through.unwrap().render(),
        })
    );
    assert_eq!(decode(&stored), Ok(fact));
}

#[test]
fn empty_recovery_baseline_round_trips_explicit_null_fields() {
    let fact = HistoryFact::RecoveryProgress {
        evidence: RecoveryEvidence {
            affected_runs: vec![],
            first_failure_observed_at: None,
            logging_restored_observed_at: None,
            failures: 0,
            recovered_records: 0,
            completed_work: 0,
            failed_work: 0,
            unknown_work: 0,
            deferred_work: 0,
            completed_effects: 0,
            failed_effects: 0,
            unresolved_effects: 0,
            missing_history: 0,
        },
        baseline: RecoveryBaseline::default(),
    };
    let stored = encode(&fact).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(
        value["baseline"],
        json!({
            "generation":null,"pending_from":null,"through":null,
        })
    );
    assert_eq!(decode(&stored), Ok(fact));
}

#[test]
fn recovery_progress_baseline_rejects_missing_invalid_and_zero_frontiers() {
    let namespace = LedgerNamespace::new([0xab; 16]);
    let valid = json!({
        "tag":"recovery_progress",
        "evidence": {
            "affected_runs":[],"first_failure_observed_at":null,
            "logging_restored_observed_at":null,"failures":0,"recovered_records":0,
            "completed_work":0,"failed_work":0,"unknown_work":0,"deferred_work":0,
            "completed_effects":0,"failed_effects":0,"unresolved_effects":0,
            "missing_history":0
        },
        "baseline": {
            "generation":0,
            "pending_from":EventId::new(namespace, 2).render(),
            "through":EventId::new(namespace, 7).render(),
        }
    });
    for invalid in [
        {
            let mut value = valid.clone();
            value.as_object_mut().unwrap().remove("baseline");
            value
        },
        {
            let mut value = valid.clone();
            value["baseline"]["pending_from"] = json!(EventId::new(namespace, 0).render());
            value
        },
        {
            let mut value = valid.clone();
            value["baseline"]["through"] = json!(EventId::new(namespace, 0).render());
            value
        },
        {
            let mut value = valid.clone();
            value["baseline"]["generation"] = json!(-1);
            value
        },
        {
            let mut value = valid.clone();
            value["baseline"]["through"] = json!(null);
            value
        },
        {
            let mut value = valid.clone();
            value["baseline"]["pending_from"] =
                json!(EventId::new(LedgerNamespace::new([0x91; 16]), 2).render());
            value
        },
    ] {
        assert!(decode(&invalid.to_string()).is_err(), "{invalid}");
    }
}
