use super::*;
use serde_json::{Value, json};
use std::path::PathBuf;
use vpt_domain::history::{EventDetail, EventPathKey, ExportFailureId};
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, FailureWindow, HookFailureClass, RecoveryEvidence,
    RunKind, RunOutcome, WorkOutcome,
};
use vpt_domain::sweep::DeferralReason;

#[test]
fn analysis_state_history_names_the_recording_and_both_states() {
    assert_fact(
        HistoryFact::AnalysisStateChanged {
            recording: recording(),
            from: vpt_domain::stage::StageState::Disabled,
            to: vpt_domain::stage::StageState::Pending,
        },
        json!({"tag":"analysis_state_changed", "recording":recording().as_str(),
        "from":"disabled", "to":"pending"}),
    );
}

#[test]
fn deferred_work_has_distinct_outgoing_history_outcome() {
    let result = std::panic::catch_unwind(|| {
        assert_fact(
            HistoryFact::WorkStep {
                recording: None,
                outcome: WorkOutcome::Deferred,
            },
            json!({"tag":"work_step","recording":null,"outcome":"deferred"}),
        )
    });
    assert!(result.is_ok(), "deferred work outcome is unsupported");
}

#[test]
fn missing_retention_history_has_exact_outgoing_fact_without_invented_identity() {
    assert_fact(
        HistoryFact::MissingRetentionHistory { intent_id: 71 },
        json!({"tag":"missing_retention_history","intent_id":71}),
    );
}

#[test]
fn deferred_event_history_keeps_reason_count_and_source_path() {
    let mut value = event(EventKind::Deferred, EventState::NeedsAttention);
    value.detail = EventDetail::Deferred {
        reason: DeferralReason::AudioTooLarge,
        count: 9,
    };
    value.paths.push(EventPath {
        key: EventPathKey::Source,
        path: PathBuf::from("/owned/recording.m4a"),
    });
    let result = std::panic::catch_unwind(|| {
        assert_fact(
            HistoryFact::Event(value),
            json!({"tag":"event","event":{
                "event":"deferred","state":"needs_attention","recording":null,
                "detail":"deferred 9 times: audio_too_large",
                "counts":{"numeric":0,"proper_noun":0,"low_confidence":0,"formatting":0,
                    "other":0,"unsupported":0,"agreed_unverified":0,"diagnostics":0},
                "paths":{"source":"/owned/recording.m4a"},
                "occurred_at":"1970-01-01T00:00:00Z"
            }}),
        )
    });
    assert!(result.is_ok(), "deferred event metadata is unsupported");
}

fn assert_fact(fact: HistoryFact, expected: Value) {
    let record = record(fact);
    let line = encode_history(&record, UtcInstant { secs: 12 }).unwrap();
    assert!(!line.contains('\n'));
    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap(),
        json!({
            "schema":"vpt.log/1",
            "event_id":record.id.render(),
            "run_id":record.run.render(),
            "sequence":2,
            "occurred_at":"1970-01-01T00:00:11Z",
            "appended_at":"1970-01-01T00:00:12Z",
            "fact":expected
        })
    );
}

fn evidence() -> RecoveryEvidence {
    RecoveryEvidence {
        affected_runs: vec![record(HistoryFact::RunStarted(RunKind::Ingest)).run],
        first_failure_observed_at: Some(UtcInstant { secs: 13 }),
        logging_restored_observed_at: Some(UtcInstant { secs: 14 }),
        failures: 1,
        recovered_records: 2,
        completed_work: 3,
        failed_work: 4,
        unknown_work: 5,
        deferred_work: 0,
        completed_effects: 6,
        failed_effects: 7,
        unresolved_effects: 8,
        missing_history: 6,
    }
}

fn evidence_json(value: &RecoveryEvidence) -> Value {
    json!({
        "affected_runs":[value.affected_runs[0].render()],
        "first_failure_observed_at":"1970-01-01T00:00:13Z",
        "logging_restored_observed_at":"1970-01-01T00:00:14Z",
        "failures":1,"recovered_records":2,
        "completed_work":3,"failed_work":4,"unknown_work":5,"deferred_work":0,
        "completed_effects":6,"failed_effects":7,"unresolved_effects":8,
        "missing_history":6
    })
}

#[test]
fn recovery_wire_separates_work_and_effect_counts() {
    let record = record(HistoryFact::RecoverySummary {
        evidence: evidence(),
        window: None,
    });
    let line = encode_history(&record, UtcInstant { secs: 12 }).unwrap();
    let value: Value = serde_json::from_str(&line).unwrap();
    let counts = &value["fact"]["evidence"];
    for (field, expected) in [
        ("completed_work", 3),
        ("failed_work", 4),
        ("unknown_work", 5),
        ("completed_effects", 6),
        ("failed_effects", 7),
        ("unresolved_effects", 8),
    ] {
        assert_eq!(counts[field], expected, "{field}");
    }
    for ambiguous in ["completed", "failed", "unresolved"] {
        assert!(counts.get(ambiguous).is_none(), "{ambiguous}");
    }
}

#[test]
fn recovery_wire_names_deferred_work_separately() {
    let record = record(HistoryFact::RecoverySummary {
        evidence: evidence(),
        window: None,
    });
    let line = encode_history(&record, UtcInstant { secs: 12 }).unwrap();
    let value: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(value["fact"]["evidence"]["deferred_work"], 0);
    assert_eq!(value["fact"]["evidence"]["unknown_work"], 5);
}

#[test]
fn recovery_progress_wire_exposes_fixed_backlog_baseline() {
    let example = record(HistoryFact::RunStarted(RunKind::Recover));
    let baseline = vpt_domain::run::RecoveryBaseline {
        generation: Some(3),
        pending_from: Some(example.id),
        through: Some(example.id),
    };
    assert_fact(
        HistoryFact::RecoveryProgress {
            evidence: evidence(),
            baseline,
        },
        json!({
            "tag":"recovery_progress",
            "evidence":evidence_json(&evidence()),
            "baseline":{
                "generation":3,
                "pending_from":example.id.render(),
                "through":example.id.render(),
            }
        }),
    );
}

#[test]
fn history_document_pins_every_closed_fact_variant() {
    for (kind, name) in [
        (RunKind::Ingest, "ingest"),
        (RunKind::Recover, "recover"),
        (RunKind::Publish, "publish"),
        (RunKind::Retain, "retain"),
        (RunKind::Transcribe, "transcribe"),
        (RunKind::Note, "note"),
    ] {
        assert_fact(
            HistoryFact::RunStarted(kind),
            json!({"tag":"run_started","kind":name}),
        );
    }
    for (outcome, name) in [
        (RunOutcome::Completed, "completed"),
        (RunOutcome::Failed, "failed"),
        (RunOutcome::Interrupted, "interrupted"),
    ] {
        assert_fact(
            HistoryFact::RunFinished(outcome),
            json!({"tag":"run_finished","outcome":name}),
        );
    }
    for (outcome, name) in [
        (WorkOutcome::Completed, "completed"),
        (WorkOutcome::Failed, "failed"),
        (WorkOutcome::Unknown, "unknown"),
    ] {
        assert_fact(
            HistoryFact::WorkStep {
                recording: Some(recording()),
                outcome,
            },
            json!({"tag":"work_step","recording":recording().as_str(),"outcome":name}),
        );
    }
    assert_fact(
        HistoryFact::WorkStep {
            recording: None,
            outcome: WorkOutcome::Unknown,
        },
        json!({"tag":"work_step","recording":null,"outcome":"unknown"}),
    );

    for (kind, name, reference, wire) in [
        (
            EffectKind::Archive,
            "archive",
            EffectReference::ArchiveTarget(PathBuf::from("/sandbox/archive")),
            json!({"tag":"archive_target","path":"/sandbox/archive"}),
        ),
        (
            EffectKind::Publication,
            "publication",
            EffectReference::PublicationTarget(PathBuf::from("/sandbox/brief")),
            json!({"tag":"publication_target","path":"/sandbox/brief"}),
        ),
        (
            EffectKind::AudioTrash,
            "audio_trash",
            EffectReference::AudioTrash {
                recording: recording(),
                path: PathBuf::from("/sandbox/audio"),
            },
            json!({"tag":"audio_trash","recording":recording().as_str(),"path":"/sandbox/audio"}),
        ),
        (
            EffectKind::RetentionTrash,
            "retention_trash",
            EffectReference::RetentionIntent {
                id: 7,
                path: PathBuf::from("/sandbox/retention"),
            },
            json!({"tag":"retention_intent","id":7,"path":"/sandbox/retention"}),
        ),
    ] {
        assert_fact(
            HistoryFact::EffectIntent { kind, reference },
            json!({"tag":"effect_intent","kind":name,"reference":wire}),
        );
    }
    let event_id = record(HistoryFact::RunStarted(RunKind::Ingest)).id;
    for (outcome, name) in [
        (EffectOutcome::Completed, "completed"),
        (EffectOutcome::Failed, "failed"),
        (EffectOutcome::Unknown, "unknown"),
    ] {
        assert_fact(
            HistoryFact::EffectObserved {
                intent: event_id,
                outcome,
            },
            json!({"tag":"effect_observed","intent":event_id.render(),"outcome":name}),
        );
    }

    for (failure, wire) in [
        (HookFailureClass::Spawn, json!({"tag":"spawn"})),
        (HookFailureClass::Io, json!({"tag":"io"})),
        (HookFailureClass::Encoding, json!({"tag":"encoding"})),
        (
            HookFailureClass::Exited(7),
            json!({"tag":"exited","code":7}),
        ),
        (
            HookFailureClass::Signaled(9),
            json!({"tag":"signaled","signal":9}),
        ),
        (
            HookFailureClass::DeadlineExceeded,
            json!({"tag":"deadline_exceeded"}),
        ),
        (HookFailureClass::Interrupted, json!({"tag":"interrupted"})),
    ] {
        assert_fact(
            HistoryFact::HookFailed {
                event: event_id,
                failure,
            },
            json!({"tag":"hook_failed","event":event_id.render(),"failure":wire}),
        );
    }

    let mut emitted = event(EventKind::BriefWritten, EventState::Done);
    emitted.recording = Some(recording());
    emitted.occurred_at = UtcInstant { secs: 11 };
    emitted.paths.push(EventPath {
        key: EventPathKey::Brief,
        path: PathBuf::from("/sandbox/brief.md"),
    });
    assert_fact(
        HistoryFact::Event(emitted),
        json!({"tag":"event","event":{
            "event":"brief_written","state":"done","recording":recording().as_str(),
            "detail":"","counts":{"numeric":0,"proper_noun":0,"low_confidence":0,
            "formatting":0,"other":0,"unsupported":0,"agreed_unverified":0,"diagnostics":0},
            "paths":{"brief":"/sandbox/brief.md"},"occurred_at":"1970-01-01T00:00:11Z"
        }}),
    );

    let recovery = evidence();
    let wire = evidence_json(&recovery);
    assert_fact(
        HistoryFact::RecoveryProgress {
            evidence: recovery.clone(),
            baseline: vpt_domain::run::RecoveryBaseline::default(),
        },
        json!({"tag":"recovery_progress","evidence":wire,"baseline":{
            "generation":null,"pending_from":null,"through":null,
        }}),
    );
    assert_fact(
        HistoryFact::RecoverySummary {
            evidence: recovery.clone(),
            window: None,
        },
        json!({"tag":"recovery_summary","evidence":wire,"window":null}),
    );
    let failure = ExportFailureId::new(recovery.affected_runs[0].namespace(), 1);
    assert_fact(
        HistoryFact::RecoverySummary {
            evidence: recovery,
            window: Some(FailureWindow {
                after: None,
                through: failure,
            }),
        },
        json!({"tag":"recovery_summary","evidence":wire,"window":{"after":null,"through":failure.render()}}),
    );
}

#[test]
fn note_move_history_names_the_recording_and_both_paths() {
    let id = vpt_domain::identity::RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    assert_fact(
        HistoryFact::NoteMoved {
            recording: id.clone(),
            from: "/notes/old.md".into(),
            to: "/notes/new.md".into(),
        },
        json!({"tag":"note_moved", "recording":id.as_str(), "from":"/notes/old.md", "to":"/notes/new.md"}),
    );
}

#[test]
fn flag_resolution_history_contains_only_identity_and_kind() {
    let id = vpt_domain::identity::RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let flag = vpt_domain::digest::Sha256Digest([3; 32]);
    for kind in [
        vpt_domain::review::ResolutionKind::Confirmed,
        vpt_domain::review::ResolutionKind::Corrected,
        vpt_domain::review::ResolutionKind::Dismissed,
    ] {
        assert_fact(
            HistoryFact::FlagResolved {
                recording: id.clone(),
                flag,
                resolution: kind,
            },
            json!({"tag":"flag_resolved", "recording":id.as_str(), "flag":flag.hex(), "resolution":kind.as_str()}),
        );
    }
}

#[test]
fn occasion_history_has_its_own_identity_without_private_provider_fields() {
    let id = vpt_domain::occasion::OccasionId::parse("2026-09-30T230000-abcdefabcdef").unwrap();
    assert_fact(
        HistoryFact::OccasionSaved {
            occasion: id.clone(),
        },
        json!({"tag":"occasion_saved", "occasion":id.as_str()}),
    );
    assert_fact(
        HistoryFact::RunStarted(RunKind::Brief),
        json!({"tag":"run_started", "kind":"brief"}),
    );
}

#[test]
fn redaction_runs_have_a_typed_public_history_name() {
    assert_fact(
        HistoryFact::RunStarted(RunKind::Redact),
        json!({"tag":"run_started", "kind":"redact"}),
    );
}
