use crate::compose::{errors::ledger_error, history::commit};
mod brief_report;
mod entry;
mod errors;
mod repair;
mod saved;
pub(crate) mod vocabulary;
use std::collections::BTreeSet;
use vpt_adapters::SystemClock;
use vpt_application::ports::{
    BriefLedger, Clock, EngineOutputLedger, EventSink, NoteLedger, PublicationJournal, RunHistory,
    SinkWarning, Stores, VocabularyLedger, WorkDelta,
};
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};
use vpt_protocol::error::ErrorDocument;

pub(crate) fn repair<L, S, E>(
    ledger: &L,
    stores: &S,
    settings: &vpt_application::Settings,
    sink: &mut E,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<Vec<String>, Box<ErrorDocument>>
where
    L: PublicationJournal
        + RunHistory
        + EngineOutputLedger
        + NoteLedger
        + BriefLedger
        + VocabularyLedger,
    S: Stores,
    E: EventSink,
{
    repair::run(ledger, stores, settings, sink, warnings, false)
}

pub(crate) fn repair_upcoming<L, S, E>(
    ledger: &L,
    stores: &S,
    settings: &vpt_application::Settings,
    sink: &mut E,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<Vec<String>, Box<ErrorDocument>>
where
    L: PublicationJournal
        + RunHistory
        + EngineOutputLedger
        + NoteLedger
        + BriefLedger
        + VocabularyLedger,
    S: Stores,
    E: EventSink,
{
    repair::run(ledger, stores, settings, sink, warnings, true)
}

pub(crate) fn merge_completed(error: &mut ErrorDocument, ids: impl IntoIterator<Item = String>) {
    for id in ids {
        if !error.completed.contains(&id) {
            error.completed.push(id);
        }
    }
}
