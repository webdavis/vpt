use std::collections::HashMap;
use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{
    FailureWindow, HistoryFact, RecoveryBaseline, RecoveryEvidence, RecoveryView, RunKind,
};

mod counts;

#[derive(Default)]
struct RunState {
    kind: Option<RunKind>,
    finished: bool,
    work_bearing: bool,
    summary: bool,
    latest_activity: u64,
    last_summary: u64,
}

pub(super) struct RecoveryPlan {
    pub(super) evidence: RecoveryEvidence,
    pub(super) window: Option<FailureWindow>,
    pub(super) resume_run: Option<RunId>,
}

impl RecoveryPlan {
    pub(super) fn from_view(current: &RecoveryView) -> Result<Option<Self>, LedgerError> {
        let mut runs: HashMap<RunId, RunState> = HashMap::new();
        let mut intents: HashMap<EventId, RunId> = HashMap::new();
        for record in &current.records {
            if let HistoryFact::EffectIntent { .. } = &record.fact {
                intents.insert(record.id, record.run);
            }
        }
        for record in &current.records {
            let sequence = record.id.sequence();
            {
                let state = runs.entry(record.run).or_default();
                match &record.fact {
                    HistoryFact::RunStarted(kind) => {
                        state.kind = Some(*kind);
                        state.latest_activity = sequence;
                    }
                    HistoryFact::RunFinished(_) => {
                        state.finished = true;
                        state.latest_activity = sequence;
                    }
                    HistoryFact::RecoveryProgress { .. } => {}
                    HistoryFact::MissingRetentionHistory { .. } => {
                        state.work_bearing = true;
                        state.latest_activity = sequence;
                    }
                    HistoryFact::RecoverySummary { .. } => state.summary = true,
                    HistoryFact::EffectObserved { .. }
                    | HistoryFact::WorkStep { .. }
                    | HistoryFact::OccasionSaved { .. }
                    | HistoryFact::BriefSaved { .. }
                    | HistoryFact::BriefPublished { .. }
                    | HistoryFact::NoteMoved { .. }
                    | HistoryFact::TagRejected { .. }
                    | HistoryFact::TagRenamed { .. }
                    | HistoryFact::AnalysisStateChanged { .. }
                    | HistoryFact::FlagResolved { .. }
                    | HistoryFact::EffectIntent { .. } => {
                        state.work_bearing = true;
                        state.latest_activity = sequence;
                    }
                    HistoryFact::Event(_) | HistoryFact::HookFailed { .. } => {
                        state.latest_activity = sequence;
                    }
                }
            }
            if let HistoryFact::RecoverySummary { evidence, .. } = &record.fact {
                for run in &evidence.affected_runs {
                    runs.entry(*run).or_default().last_summary = sequence;
                }
            }
            if let HistoryFact::EffectObserved { intent, .. } = &record.fact
                && let Some(owner) = intents.get(intent)
            {
                runs.entry(*owner).or_default().latest_activity = sequence;
            }
        }
        let threshold = current.summarized_through.map_or(0, |id| id.sequence());
        let failures: Vec<_> = current
            .failures
            .iter()
            .filter(|saved| saved.id.sequence() > threshold)
            .collect();
        let window = failures.last().map(|saved| FailureWindow {
            after: current.summarized_through,
            through: saved.id,
        });
        let mut affected = Vec::new();
        for (run, state) in &runs {
            let changed = state.last_summary > 0 && state.latest_activity > state.last_summary;
            let unfinished = state.last_summary == 0 && !state.finished;
            let selected = if state.kind == Some(RunKind::Recover) {
                state.work_bearing && (state.last_summary == 0 || changed)
            } else {
                state.kind.is_some() && (changed || unfinished)
            };
            if selected {
                affected.push(*run);
            }
        }
        for saved in &failures {
            if let Some(record) = current
                .records
                .iter()
                .find(|record| record.id == saved.failure.event)
            {
                match &record.fact {
                    HistoryFact::RecoveryProgress { evidence, .. }
                    | HistoryFact::RecoverySummary { evidence, .. } => {
                        affected.extend(&evidence.affected_runs);
                    }
                    _ if runs.get(&record.run).is_some_and(|state| {
                        state.kind != Some(RunKind::Recover) || state.work_bearing
                    }) =>
                    {
                        affected.push(record.run)
                    }
                    _ => {}
                }
            }
        }
        affected.sort_by_key(|run| (run.namespace().bytes(), run.sequence()));
        affected.dedup();
        let resume_run = runs
            .iter()
            .filter(|(_, state)| {
                state.kind == Some(RunKind::Recover)
                    && !state.summary
                    && !state.finished
                    && !state.work_bearing
            })
            .map(|(run, _)| *run)
            .max_by_key(|run| run.sequence());
        if window.is_none() && affected.is_empty() && resume_run.is_none() {
            return Ok(None);
        }
        let counts = counts::from_records(&current.records, &affected)?;
        Ok(Some(Self {
            evidence: RecoveryEvidence {
                affected_runs: affected,
                first_failure_observed_at: failures.first().map(|saved| saved.failure.observed_at),
                logging_restored_observed_at: None,
                failures: count(failures.len())?,
                recovered_records: 0,
                completed_work: counts.completed_work,
                failed_work: counts.failed_work,
                unknown_work: counts.unknown_work,
                deferred_work: counts.deferred_work,
                completed_effects: counts.completed_effects,
                failed_effects: counts.failed_effects,
                unresolved_effects: counts.unresolved_effects,
                missing_history: counts.missing_history,
            },
            window,
            resume_run,
        }))
    }

    pub(super) fn baseline_for(
        current: &RecoveryView,
        run: RunId,
    ) -> Result<Option<RecoveryBaseline>, LedgerError> {
        let mut saved = None;
        for record in current.records.iter().filter(|record| record.run == run) {
            if let HistoryFact::RecoveryProgress { baseline, .. } = &record.fact {
                if !baseline.is_valid_for(run.namespace()) {
                    return Err(LedgerError::Corrupt("invalid recovery baseline".into()));
                }
                if saved.is_some_and(|first| first != *baseline) {
                    return Err(LedgerError::Corrupt("recovery baseline changed".into()));
                }
                saved = Some(*baseline);
            }
        }
        Ok(saved)
    }

    pub(super) fn snapshot_baseline(
        current: &RecoveryView,
        run: RunId,
    ) -> Result<RecoveryBaseline, LedgerError> {
        let baseline = RecoveryBaseline {
            generation: current.active_cursor.map(|cursor| cursor.generation),
            pending_from: current
                .records
                .iter()
                .find(|record| record.appended_at.is_none())
                .map(|record| record.id),
            through: current.records.last().map(|record| record.id),
        };
        baseline
            .is_valid_for(run.namespace())
            .then_some(baseline)
            .ok_or_else(|| LedgerError::Corrupt("invalid recovery baseline".into()))
    }

    pub(super) fn confirmed_recovered(
        current: &RecoveryView,
        baseline: RecoveryBaseline,
    ) -> Result<u32, LedgerError> {
        let generation = current.active_cursor.map(|cursor| cursor.generation);
        let restarted = match (baseline.generation, generation) {
            (None, None) | (None, Some(0)) => false,
            (None, Some(_)) => true,
            (Some(_), None) => {
                return Err(LedgerError::Corrupt("recovery cursor disappeared".into()));
            }
            (Some(before), Some(after)) if after < before => {
                return Err(LedgerError::Corrupt("recovery cursor regressed".into()));
            }
            (Some(before), Some(after)) => after > before,
        };
        let Some(through) = baseline.through else {
            return Ok(0);
        };
        let first = match (restarted, baseline.pending_from) {
            (true, _) => 1,
            (false, Some(from)) => from.sequence(),
            (false, None) => return Ok(0),
        };
        count(
            current
                .records
                .iter()
                .filter(|record| {
                    record.id.namespace() == through.namespace()
                        && record.id.sequence() >= first
                        && record.id.sequence() <= through.sequence()
                        && record.export_position.is_some()
                })
                .count(),
        )
    }
}

fn count(value: usize) -> Result<u32, LedgerError> {
    u32::try_from(value).map_err(|_| LedgerError::Corrupt("recovery count overflow".into()))
}
