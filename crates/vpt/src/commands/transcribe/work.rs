mod finish;
mod output;
mod publish;
mod publish_output;
mod reconcile;
use crate::{cli::output::Outcome, compose::history::commit};
use std::collections::BTreeSet;
use vpt_adapters::{FilesystemStores, GuardedEngines, Roots, SqliteLedger, SystemClock};
use vpt_application::transcribe::{TranscriptionError, TranscriptionInput, save, transcribe};
use vpt_application::{
    Settings,
    ports::{Clock, EventSink, RecordingRecord, RunHistory, SinkWarning},
};
use vpt_domain::run::{HistoryFact, WorkOutcome};
use vpt_protocol::error::ErrorDocument;

pub(crate) struct Context<'a, E> {
    pub mentions: Option<&'a crate::compose::mentions::Context>,
    pub notices: Option<&'a mut vpt_application::review::alerts::ReviewQueue>,
    pub ledger: &'a SqliteLedger,
    pub stores: &'a FilesystemStores,
    pub settings: &'a Settings,
    pub roots: &'a Roots,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
}

pub(crate) fn run<E: EventSink>(
    mut context: Context<'_, E>,
    record: RecordingRecord,
    duration: f64,
    run: vpt_domain::history::RunId,
) -> Result<Outcome, Box<ErrorDocument>> {
    let result = transcribe(
        &context.settings.engines,
        &GuardedEngines::new(
            context.settings.helper_path.clone(),
            record.digest,
            context.settings.source.max_audio_bytes,
        ),
        &TranscriptionInput {
            audio: &record.audio_path,
            out_dir: &context.roots.stores.engine_outputs,
            language: None,
            duration,
        },
    );
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            let interrupted = matches!(error, TranscriptionError::Interrupted(_));
            return finish::failure(
                &mut context,
                run,
                &record.id,
                super::errors::engine(error),
                interrupted,
            );
        }
    };
    let engine_diagnostics = output::engine_diagnostics(&result);
    let completed = (|| {
        let uncertainty = match reconcile::classify(&result, context.settings) {
            Ok(uncertainty) => uncertainty,
            Err(error) => return finish::failure(&mut context, run, &record.id, error, false),
        };
        let diagnostics: Vec<_> = uncertainty
            .diagnostics
            .iter()
            .map(|value| value.as_str())
            .collect();
        let engine_name = result.record.transcript.engine.name.clone();
        let language = result.language.clone();
        let engines = output::engines(&result);
        let delta = match save::prepare(
            result,
            save::AcceptedInput {
                recording: &record.id,
                duration,
                at: SystemClock.now(),
                root: &context.roots.stores.engine_outputs,
                uncertainty,
            },
            context.ledger,
            context.stores,
        ) {
            Ok(delta) => delta,
            Err(error) => {
                return finish::failure(
                    &mut context,
                    run,
                    &record.id,
                    super::errors::save(error),
                    false,
                );
            }
        };
        let paths: Vec<_> = delta
            .engine_outputs
            .iter()
            .map(|output| output.target.clone())
            .collect();
        commit(
            context.ledger.commit_step(
                run,
                &delta,
                &[(
                    SystemClock.now(),
                    HistoryFact::WorkStep {
                        recording: Some(record.id.clone()),
                        outcome: WorkOutcome::Completed,
                    },
                )],
            ),
            context.sink,
            context.warnings,
        )?;
        let published = (|| {
            let published = publish::run(&mut context, &record.id, run)?;
            let record = super::observe::record(context.ledger, &record.id)?;
            output::success(
                &record,
                &engine_name,
                &language,
                &engines,
                &paths,
                &diagnostics,
                &engine_diagnostics,
            )
            .map(|outcome| published.attach(outcome))
        })();
        published.map_err(|mut error: Box<ErrorDocument>| {
            if !error.completed.contains(&record.id.to_string()) {
                error.completed.push(record.id.to_string());
            }
            error
        })
    })();
    completed.map_err(|mut error| {
        error.diagnostics.extend(engine_diagnostics);
        error
    })
}
