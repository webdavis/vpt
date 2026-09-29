use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PendingPair {
    pub run: RunId,
    pub intents: [i64; 2],
    pub effects: [EventId; 2],
}

pub(crate) trait RetentionPairFixture {
    fn seed_retention_pair(&self) -> PendingPair;
}

pub(crate) fn pair_through_ports<L: RunHistory + RetentionJournal>(ledger: &L) -> PendingPair {
    let run = begin(ledger).run;
    let new = intent();
    let intents = [
        ledger.record_intent(&new).unwrap(),
        ledger.record_intent(&new).unwrap(),
    ];
    let saved = committed(ledger.commit_step(
        run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: vec![record(120)],
                ..Default::default()
            },
            ..Default::default()
        },
        &intents.map(|id| {
            (
                new.recorded_at,
                HistoryFact::EffectIntent {
                    kind: EffectKind::RetentionTrash,
                    reference: EffectReference::RetentionIntent {
                        id: u64::try_from(id).unwrap(),
                        path: new.path.clone(),
                    },
                },
            )
        }),
    ));
    PendingPair {
        run,
        intents,
        effects: [saved.records[0].id, saved.records[1].id],
    }
}
