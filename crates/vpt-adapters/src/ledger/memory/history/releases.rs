use super::*;
use crate::ledger::release::{self as release, conflict, validate};
use vpt_application::ports::*;
use vpt_domain::run::{EffectOutcome, WorkOutcome};
impl ReleaseHistory for MemoryLedger {
    fn prepare_release(&self, run: RunId, draft: &NewRelease) -> HistoryWrite<PreparedRelease> {
        self.history_write(|state| {
            if !state.history.has_run(run) {
                return Err(conflict());
            }
            let sequence = state.next_release.checked_add(1).ok_or_else(conflict)?;
            let recording = state
                .recordings
                .iter()
                .find(|r| r.id == draft.recording)
                .ok_or_else(conflict)?;
            let row = validate::prepare(draft, sequence, recording)?;
            release::codec::encode(&row.provenance)?;
            for path in [&row.destination, &row.report_path] {
                if state
                    .releases
                    .iter()
                    .any(|r| &r.destination == path || &r.report_path == path)
                    || state.publications.iter().any(|p| &p.target == path)
                    || state.notes.iter().any(|p| &p.target == path)
                    || state.briefs.iter().any(|p| &p.target == path)
                    || state.engine_outputs.iter().any(|p| &p.target == path)
                    || state.vocabulary.iter().any(|p| &p.entry.target == path)
                    || state
                        .intents
                        .iter()
                        .any(|(p, done)| done.is_none() && &p.path == path)
                {
                    return Err(conflict());
                }
            }
            let [a, b] = release::intents(&row);
            let a = state.history.append(run, row.created_at, a)?;
            let b = state.history.append(run, row.created_at, b)?;
            state.release_payloads.push(ReleasePayload {
                release: row.clone(),
                text: draft.text.clone(),
                report: draft.report.clone(),
                intents: [a.id, b.id],
            });
            state.releases.push(row.clone());
            state.next_release = sequence;
            Ok(PreparedRelease {
                release: row,
                receipt: HistoryReceipt {
                    run,
                    records: vec![a, b],
                    completed: vec![],
                },
            })
        })
    }
    fn complete_release(
        &self,
        run: RunId,
        sequence: i64,
        expected: ReleaseVersion,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|state| {
            validate::instant(at)?;
            if !state.history.has_run(run) {
                return Err(conflict());
            }
            let row = state
                .releases
                .iter()
                .find(|r| r.sequence == sequence)
                .ok_or_else(conflict)?;
            if row.version != expected {
                return Err(conflict());
            }
            let mut receipt = HistoryReceipt {
                run,
                records: vec![],
                completed: vec![row.recording.clone()],
            };
            if row.finalized_at.is_some() {
                return Ok(receipt);
            }
            let payload = state
                .release_payloads
                .iter()
                .find(|p| p.release.sequence == sequence)
                .ok_or_else(conflict)?;
            for (intent, fact) in payload.intents.iter().zip(release::intents(row)) {
                if !state
                    .history
                    .records
                    .iter()
                    .any(|r| r.id == *intent && r.fact == fact)
                {
                    return Err(conflict());
                }
            }
            let facts = payload.intents.map(|intent| HistoryFact::EffectObserved {
                intent,
                outcome: EffectOutcome::Completed,
            });
            for fact in facts.into_iter().chain([HistoryFact::WorkStep {
                recording: Some(row.recording.clone()),
                outcome: WorkOutcome::Completed,
            }]) {
                receipt.records.push(state.history.append(run, at, fact)?);
            }
            state
                .releases
                .iter_mut()
                .find(|r| r.sequence == sequence)
                .ok_or_else(conflict)?
                .finalized_at = Some(at);
            state
                .release_payloads
                .retain(|p| p.release.sequence != sequence);
            Ok(receipt)
        })
    }
}
