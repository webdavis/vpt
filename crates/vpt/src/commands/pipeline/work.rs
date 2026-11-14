mod analysis;
mod child;
mod eligibility;
mod initial;
mod notices;
mod sharing;
mod stages;
mod verification;
use super::Report;
use crate::compose::{
    config_refusal,
    errors::{contained_error, ledger_error},
    history::commit,
};
use std::collections::BTreeSet;
use vpt_adapters::{
    ClonefileArchive, FilesystemStores, HelperClient, RootDir, Roots, SqliteLedger, SystemClock,
    VoiceMemosStore,
};
use vpt_application::{Mode, Settings, ports::*, review::alerts::ReviewQueue};
use vpt_domain::{history::RunId, run::HistoryFact};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) struct Context<'a, E> {
    pub ledger: &'a SqliteLedger,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
    pub settings: &'a Settings,
    pub roots: &'a Roots,
    pub state: &'a RootDir,
    pub helper: &'a HelperClient,
    pub resend_refused: bool,
}

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    run: RunId,
    report: &mut Report,
    notices: &mut ReviewQueue,
) -> Result<(), Box<ErrorDocument>> {
    let recorder = VoiceMemosStore::open(&context.roots.recordings_dir)
        .map_err(|error| {
            crate::commands::ingest::startup_failure::source(
                context.ledger,
                context.sink,
                error,
                context.warnings,
            )
        })?
        .with_titles_root(context.state, context.settings.source.read_titles);
    context.roots.create_leaves().map_err(|error| {
        config_refusal::in_run(context.ledger, context.sink, run, error, context.warnings)
    })?;
    let archive = ClonefileArchive::open(&context.roots.stores.audio)
        .map_err(|error| Box::new(contained_error(error)))?;
    report.ingestion = crate::commands::ingest::runtime::execute(
        &recorder,
        &archive,
        context.ledger,
        context.helper,
        context.sink,
        &context.settings.source,
        &Mode::default(),
    )
    .map_err(ingest_error)?;
    context
        .warnings
        .extend(report.ingestion.warnings.iter().copied());
    let paths =
        vpt_domain::layout::StoreKey::all().map(|key| context.roots.stores.get(key).to_path_buf());
    let stores = FilesystemStores::open(&paths)
        .map_err(|error| Box::new(contained_error(error)))?
        .with_private_roots(std::slice::from_ref(&context.roots.stores.engine_outputs));
    let staged = stages::run(context, &stores, run, report, notices);
    let sent = crate::compose::actions::Delivery {
        ledger: context.ledger,
        settings: context.settings,
        sink: context.sink,
        warnings: context.warnings,
        run,
    }
    .send(None, context.resend_refused)?;
    report.actions = (!context.settings.actions_command.is_empty()).then_some(sent);
    staged?;
    super::retention::run(context, &stores, run, report)
}

pub(super) fn ingest_error(error: Box<vpt_application::IngestError>) -> Box<ErrorDocument> {
    let mut diagnostics = error.log;
    diagnostics.extend(
        error
            .warnings
            .into_iter()
            .map(crate::compose::events::warning_text)
            .map(str::to_owned),
    );
    Box::new(
        crate::commands::ingest::errors::document(error.failure)
            .completed(error.completed.iter().map(ToString::to_string).collect())
            .diagnostics(diagnostics),
    )
}

pub(super) fn finish<E: EventSink>(
    context: &mut Context<'_, E>,
    run: RunId,
    notices: &ReviewQueue,
    report: &Report,
    result: Result<(), Box<ErrorDocument>>,
) -> Result<(), Box<ErrorDocument>> {
    if result.as_ref().is_err_and(|error| {
        error.kind == ErrorKind::Ledger || error.rule.as_deref() == Some("unsafe_state")
    }) {
        return result;
    }
    let at = SystemClock.now();
    let mut facts: Vec<_> = notices
        .events(context.settings.alert_aggregate_after, at)
        .map_err(|_| {
            Box::new(ErrorDocument::new(
                ErrorKind::Ledger,
                "review alert capacity exceeded",
            ))
        })?
        .into_iter()
        .map(|event| (at, HistoryFact::Event(event)))
        .collect();
    if let Some(event) = report
        .retention
        .event(at, report.retention_failed)
        .map_err(|error| Box::new(ledger_error(error)))?
    {
        facts.push((at, HistoryFact::Event(event)));
    }
    facts.push((
        at,
        HistoryFact::RunFinished(crate::compose::run_outcome::from_result(&result)),
    ));
    commit(
        context
            .ledger
            .commit_step(run, &WorkDelta::default(), &facts),
        context.sink,
        context.warnings,
    )
    .map_err(|mut error| {
        if let Err(original) = &result {
            for id in &original.completed {
                if !error.completed.contains(id) {
                    error.completed.push(id.clone());
                }
            }
        }
        error
    })?;
    result
}
