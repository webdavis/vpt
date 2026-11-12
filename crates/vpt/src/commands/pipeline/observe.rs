use super::{Report, output};
use crate::{
    cli::output::Outcome,
    compose::{
        RuntimeLedger,
        errors::{contained_error, ledger_error},
    },
};
use vpt_adapters::{ClonefileArchive, HelperClient, Roots, VoiceMemosStore};
use vpt_application::{Mode, Settings, ports::RecordingLedger};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(settings: &Settings, roots: &Roots) -> Result<Outcome, Box<ErrorDocument>> {
    let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
    let recorder = VoiceMemosStore::open(&roots.recordings_dir)
        .map_err(|error| Box::new(crate::commands::ingest::errors::source(error)))?;
    let archive = ClonefileArchive::open_read_only(&roots.stores.audio)
        .map_err(|error| Box::new(contained_error(error)))?;
    let helper = HelperClient::new(settings.helper_path.clone());
    let mut sink = crate::compose::events::Observation;
    let mode = Mode {
        dry_run: true,
        once: None,
        import: None,
    };
    let ingestion = match &ledger {
        RuntimeLedger::Sqlite(ledger) => crate::commands::ingest::runtime::execute(
            &recorder,
            &archive,
            ledger,
            &helper,
            &mut sink,
            &settings.source,
            &mode,
        ),
        RuntimeLedger::Empty(ledger) => crate::commands::ingest::runtime::execute(
            &recorder,
            &archive,
            ledger,
            &helper,
            &mut sink,
            &settings.source,
            &mode,
        ),
    }
    .map_err(super::work::ingest_error)?;
    let mut records = ledger
        .recordings()
        .map_err(|error| Box::new(ledger_error(error)))?;
    records.sort_by(|a, b| {
        (a.captured_at.secs, a.id.as_str()).cmp(&(b.captured_at.secs, b.id.as_str()))
    });
    if records.len() > 65_536 {
        return Err(Box::new(vpt_protocol::error::ErrorDocument::new(
            vpt_protocol::error::ErrorKind::Ledger,
            "pipeline recording capacity exceeded",
        )));
    }
    let stores = vpt_adapters::FilesystemStores::open_read_only(&[
        roots.stores.transcripts.clone(),
        roots.stores.analysis.clone(),
    ])
    .map_err(|error| Box::new(contained_error(error)))?;
    let mentions = crate::compose::mentions::Context::read(settings, roots)?;
    let mut plan = Vec::new();
    for record in &records {
        if let Some(stage) =
            super::selection::next(&ledger, settings, roots, &stores, record, &mentions)?
        {
            plan.push((record.id.clone(), stage));
        } else if let RuntimeLedger::Sqlite(ledger) = &ledger
            && let Some(stage) =
                crate::commands::redact::plan::next(ledger, settings, roots, record)?
        {
            plan.push((
                record.id.clone(),
                vpt_application::pipeline::NextStage::Redact(stage),
            ));
        }
    }
    let retention = super::retention::observe(&ledger, settings, roots, &helper)?;
    output::success(
        &Report {
            ingestion,
            plan,
            retention,
            ..Default::default()
        },
        true,
    )
}
