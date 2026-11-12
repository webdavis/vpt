mod values;
use super::{
    corrupt, event, event_id, failure_id, field, optional, recording, signed, string, unsigned,
};
use serde_json::{Value, json};
use std::path::PathBuf;
pub(in crate::ledger::sqlite::history) use values::run_kind;
use values::*;
use vpt_application::ports::LedgerError;
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, FailureWindow, HistoryFact, HookFailureClass,
    RunKind, RunOutcome, WorkOutcome,
};

mod encode;
pub(super) use encode::encode;

pub(super) fn decode(value: &Value) -> Result<HistoryFact, LedgerError> {
    match string(value, "tag")? {
        "brief_saved" => Ok(HistoryFact::BriefSaved {
            occasion: vpt_domain::occasion::OccasionId::parse(string(value, "occasion")?)
                .map_err(|_| corrupt())?,
        }),
        "brief_published" => Ok(HistoryFact::BriefPublished {
            occasion: vpt_domain::occasion::OccasionId::parse(string(value, "occasion")?)
                .map_err(|_| corrupt())?,
        }),
        "occasion_saved" => Ok(HistoryFact::OccasionSaved {
            occasion: vpt_domain::occasion::OccasionId::parse(string(value, "occasion")?)
                .map_err(|_| corrupt())?,
        }),
        "analysis_state_changed" => Ok(HistoryFact::AnalysisStateChanged {
            recording: recording(value, "recording")?.ok_or_else(corrupt)?,
            from: vpt_domain::stage::StageState::parse(string(value, "from")?)
                .ok_or_else(corrupt)?,
            to: vpt_domain::stage::StageState::parse(string(value, "to")?).ok_or_else(corrupt)?,
        }),
        "tag_rejected" => Ok(HistoryFact::TagRejected {
            recording: recording(value, "recording")?.ok_or_else(corrupt)?,
            index: unsigned(value, "index")?
                .try_into()
                .ok()
                .filter(|index| *index < 256)
                .ok_or_else(corrupt)?,
            reason: vpt_domain::tags::Rejection::parse(string(value, "reason")?)
                .ok_or_else(corrupt)?,
        }),
        "tag_renamed" => Ok(HistoryFact::TagRenamed {
            recording: recording(value, "recording")?.ok_or_else(corrupt)?,
            index: unsigned(value, "index")?
                .try_into()
                .ok()
                .filter(|index| *index < 256)
                .ok_or_else(corrupt)?,
        }),
        "flag_resolved" => Ok(HistoryFact::FlagResolved {
            recording: recording(value, "recording")?.ok_or_else(corrupt)?,
            flag: vpt_domain::digest::Sha256Digest::from_hex(string(value, "flag")?)
                .ok_or_else(corrupt)?,
            resolution: vpt_domain::review::ResolutionKind::parse(string(value, "resolution")?)
                .ok_or_else(corrupt)?,
        }),
        "note_moved" => Ok(HistoryFact::NoteMoved {
            recording: recording(value, "recording")?.ok_or_else(corrupt)?,
            from: string(value, "from")?.into(),
            to: string(value, "to")?.into(),
        }),
        "run_started" => Ok(HistoryFact::RunStarted(parse_run_kind(string(
            value, "kind",
        )?)?)),
        "run_finished" => Ok(HistoryFact::RunFinished(match string(value, "outcome")? {
            "completed" => RunOutcome::Completed,
            "failed" => RunOutcome::Failed,
            "interrupted" => RunOutcome::Interrupted,
            _ => return Err(corrupt()),
        })),
        "work" => Ok(HistoryFact::WorkStep {
            recording: recording(value, "recording")?,
            outcome: match string(value, "outcome")? {
                "completed" => WorkOutcome::Completed,
                "failed" => WorkOutcome::Failed,
                "unknown" => WorkOutcome::Unknown,
                "deferred" => WorkOutcome::Deferred,
                _ => return Err(corrupt()),
            },
        }),
        "effect_intent" => {
            let kind = match string(value, "kind")? {
                "archive" => EffectKind::Archive,
                "publication" => EffectKind::Publication,
                "audio_trash" => EffectKind::AudioTrash,
                "retention_trash" => EffectKind::RetentionTrash,
                _ => return Err(corrupt()),
            };
            let reference = decode_reference(field(value, "reference")?)?;
            if !reference.matches_kind(kind) {
                return Err(corrupt());
            }
            Ok(HistoryFact::EffectIntent { kind, reference })
        }
        "effect_observed" => Ok(HistoryFact::EffectObserved {
            intent: event_id(value, "intent")?,
            outcome: match string(value, "outcome")? {
                "completed" => EffectOutcome::Completed,
                "failed" => EffectOutcome::Failed,
                "unknown" => EffectOutcome::Unknown,
                _ => return Err(corrupt()),
            },
        }),
        "missing_retention_history" => {
            let intent_id = unsigned(value, "intent_id")?;
            if intent_id == 0 {
                return Err(corrupt());
            }
            Ok(HistoryFact::MissingRetentionHistory { intent_id })
        }
        "hook_failed" => Ok(HistoryFact::HookFailed {
            event: event_id(value, "event")?,
            failure: decode_hook_failure(field(value, "failure")?)?,
        }),
        "event" => Ok(HistoryFact::Event(event::decode(field(value, "event")?)?)),
        "recovery_progress" => Ok(HistoryFact::RecoveryProgress {
            evidence: super::recovery::decode(field(value, "evidence")?)?,
            baseline: super::recovery::decode_baseline(field(value, "baseline")?)?,
        }),
        "recovery_summary" => Ok(HistoryFact::RecoverySummary {
            evidence: super::recovery::decode(field(value, "evidence")?)?,
            window: optional(value, "window")?.map(decode_window).transpose()?,
        }),
        _ => Err(corrupt()),
    }
}
