use super::{LoggedEvents, required};
use std::collections::BTreeSet;
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, RunHistory, SinkOutcome, SinkStop, SinkWarning, WorkDelta,
};
use vpt_domain::run::{HistoryFact, RecoveryView, RunOutcome};
use vpt_domain::time::UtcInstant;

mod plan;
use plan::RecoveryPlan;

impl<H: RunHistory, N: FnMut() -> UtcInstant> LoggedEvents<'_, H, N> {
    pub(super) fn recover_history(&mut self, before: &RecoveryView) -> SinkOutcome {
        let mut warnings = BTreeSet::new();
        let initial = match RecoveryPlan::from_view(before) {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                if let Err(stop) = self.export_pending(None, &mut warnings) {
                    return SinkOutcome::Stop(stop);
                }
                return SinkOutcome::Continue { warnings };
            }
            Err(error) => return SinkOutcome::Stop(required(error, None, &warnings)),
        };
        let at = (self.clock)();
        let (run, baseline, started) = match initial.resume_run {
            Some(run) => {
                let saved_baseline = match RecoveryPlan::baseline_for(before, run) {
                    Ok(value) => value,
                    Err(error) => {
                        return SinkOutcome::Stop(required(error, None, &warnings));
                    }
                };
                match saved_baseline {
                    Some(baseline) => (run, baseline, None),
                    None => {
                        let baseline = match RecoveryPlan::snapshot_baseline(before, run) {
                            Ok(value) => value,
                            Err(error) => {
                                return SinkOutcome::Stop(required(error, None, &warnings));
                            }
                        };
                        let fact = HistoryFact::RecoveryProgress {
                            evidence: initial.evidence.clone(),
                            baseline,
                        };
                        let saved = match self.required_write(
                            self.history
                                .commit_step(run, &WorkDelta::default(), &[(at, fact)]),
                            None,
                            &warnings,
                        ) {
                            Ok(receipt) => receipt,
                            Err(stop) => return SinkOutcome::Stop(stop),
                        };
                        (run, baseline, Some(saved))
                    }
                }
            }
            None => {
                let started = match self.required_write(
                    self.history.begin_recovery(at, initial.evidence),
                    None,
                    &warnings,
                ) {
                    Ok(receipt) => receipt,
                    Err(stop) => return SinkOutcome::Stop(stop),
                };
                let baseline = match started.records.as_slice() {
                    [_, second] => match &second.fact {
                        HistoryFact::RecoveryProgress { baseline, .. } => *baseline,
                        _ => {
                            return SinkOutcome::Stop(required(
                                vpt_application::ports::LedgerError::Corrupt(
                                    "missing recovery progress".into(),
                                ),
                                Some(&started),
                                &warnings,
                            ));
                        }
                    },
                    _ => {
                        return SinkOutcome::Stop(required(
                            vpt_application::ports::LedgerError::Corrupt(
                                "incomplete recovery start".into(),
                            ),
                            Some(&started),
                            &warnings,
                        ));
                    }
                };
                (started.run, baseline, Some(started))
            }
        };
        if let Err(stop) = self.export_pending(started.as_ref(), &mut warnings) {
            return SinkOutcome::Stop(stop);
        }
        if warnings.contains(&SinkWarning::ExportUnavailable)
            || warnings.contains(&SinkWarning::ExportProgressUnconfirmed)
        {
            return SinkOutcome::Continue { warnings };
        }
        let current = match self.history.recovery_view() {
            Ok(view) => view,
            Err(error) => {
                return SinkOutcome::Stop(required(error, started.as_ref(), &warnings));
            }
        };
        let mut plan = match RecoveryPlan::from_view(&current) {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                return SinkOutcome::Stop(required(
                    vpt_application::ports::LedgerError::Corrupt(
                        "recovery attempt disappeared".into(),
                    ),
                    started.as_ref(),
                    &warnings,
                ));
            }
            Err(error) => {
                return SinkOutcome::Stop(required(error, started.as_ref(), &warnings));
            }
        };
        plan.evidence.recovered_records =
            match RecoveryPlan::confirmed_recovered(&current, baseline) {
                Ok(count) => count,
                Err(error) => {
                    return SinkOutcome::Stop(required(error, started.as_ref(), &warnings));
                }
            };
        if plan.evidence.first_failure_observed_at.is_some() {
            plan.evidence.logging_restored_observed_at = Some((self.clock)());
        }
        let facts = [
            (
                at,
                HistoryFact::RecoverySummary {
                    evidence: plan.evidence,
                    window: plan.window,
                },
            ),
            (at, HistoryFact::RunFinished(RunOutcome::Completed)),
        ];
        let saved = match self.required_write(
            self.history.commit_step(run, &WorkDelta::default(), &facts),
            started.as_ref(),
            &warnings,
        ) {
            Ok(receipt) => receipt,
            Err(stop) => return SinkOutcome::Stop(stop),
        };
        if let Err(stop) = self.export_pending(Some(&saved), &mut warnings) {
            return SinkOutcome::Stop(stop);
        }
        SinkOutcome::Continue { warnings }
    }

    fn required_write(
        &self,
        write: HistoryWrite<HistoryReceipt>,
        prior: Option<&HistoryReceipt>,
        warnings: &BTreeSet<SinkWarning>,
    ) -> Result<HistoryReceipt, Box<SinkStop>> {
        match write {
            HistoryWrite::Committed(receipt) => Ok(receipt),
            HistoryWrite::CommittedWithFailure { receipt, error } => {
                Err(required(error, Some(&receipt), warnings))
            }
            HistoryWrite::Unconfirmed(error) => Err(required(error, prior, warnings)),
        }
    }
}
