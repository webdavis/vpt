use super::{errors, saved, vocabulary};
use crate::compose::{errors::ledger_error, history::commit};
use std::collections::BTreeSet;
use vpt_adapters::SystemClock;
use vpt_application::{Settings, ports::*, repair_publication};
use vpt_domain::{
    history::RunId,
    occasion::OccasionId,
    run::{EffectKind, EffectOutcome, EffectReference, HistoryFact},
};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) struct Publisher<'a, L, S, E> {
    pub report_briefs: bool,
    pub ledger: &'a L,
    pub stores: &'a S,
    pub settings: &'a Settings,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
}

impl<L, S, E> Publisher<'_, L, S, E>
where
    L: RunHistory + EngineOutputLedger + NoteLedger + BriefLedger + VocabularyLedger,
    S: Stores,
    E: EventSink,
{
    pub fn publish(
        &mut self,
        run: RunId,
        entry: DirtyPublication,
    ) -> Result<Option<OccasionId>, Box<ErrorDocument>> {
        let intent = commit(
            self.ledger.commit_step(
                run,
                &WorkDelta::default(),
                &[(
                    SystemClock.now(),
                    HistoryFact::EffectIntent {
                        kind: EffectKind::Publication,
                        reference: EffectReference::PublicationTarget(entry.target.clone()),
                    },
                )],
            ),
            self.sink,
            self.warnings,
        )?;
        let intent_id = intent
            .records
            .first()
            .map(|record| record.id)
            .ok_or_else(|| {
                Box::new(ErrorDocument::new(
                    ErrorKind::Ledger,
                    "publication intent receipt is missing",
                ))
            })?;
        let result = match self.ledger.vocabulary_publication(&entry.target) {
            Ok(Some(value)) => vocabulary::publish(value, &entry, self.settings).map(|()| None),
            Ok(None) => saved::read(self.ledger, &entry.target)
                .and_then(|saved| {
                    repair_publication(self.stores, &entry, &|_| saved.bytes.clone())?;
                    Ok(saved.brief)
                })
                .map_err(|error| Box::new(errors::document(error))),
            Err(error) => Err(Box::new(ledger_error(error))),
        };
        let at = SystemClock.now();
        let mut delta = WorkDelta::default();
        let mut facts = vec![(
            at,
            HistoryFact::EffectObserved {
                intent: intent_id,
                outcome: if result.is_ok() {
                    EffectOutcome::Completed
                } else {
                    EffectOutcome::Failed
                },
            },
        )];
        let completed = if let Ok(brief) = &result {
            delta.publication_clears.push(entry.target);
            brief.as_ref().map(|brief| {
                let occasion = brief.snapshot.id.clone();
                delta.brief_published = Some(BriefPublication {
                    occasion: occasion.clone(),
                    expected: brief.version,
                    at,
                });
                facts.push((
                    at,
                    HistoryFact::BriefPublished {
                        occasion: occasion.clone(),
                    },
                ));
                if self.report_briefs {
                    facts.push((
                        at,
                        HistoryFact::Event(super::brief_report::event(brief, at)),
                    ));
                }
                occasion
            })
        } else {
            None
        };
        commit(
            self.ledger.commit_step(run, &delta, &facts),
            self.sink,
            self.warnings,
        )?;
        result.map(|_| completed)
    }
}
