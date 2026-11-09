use crate::{
    cli::output::Outcome,
    compose::{history::commit, publication},
};
use std::collections::BTreeSet;
use vpt_adapters::{FilesystemStores, SqliteLedger, SystemClock, known_terms};
use vpt_application::{Settings, ports::*};
use vpt_domain::run::*;
use vpt_protocol::{error::ErrorDocument, result::document};

pub(super) fn run(
    ledger: &SqliteLedger,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
    settings: &Settings,
    term: &str,
) -> Result<Outcome, Box<ErrorDocument>> {
    let run = commit(
        ledger.begin_run(RunKind::Confirm, SystemClock.now()),
        sink,
        warnings,
    )?
    .run;
    let path = &settings.reconcile.known_terms_path;
    let result = (|| {
        let update =
            known_terms::prepare(path, term).map_err(publication::vocabulary::term_error)?;
        let added = update.added();
        if added {
            let at = SystemClock.now();
            let saved = VocabularyPublication {
                kind: vpt_application::ports::VocabularyKind::Terms,
                entry: update.publication(at),
                bytes: update.content().to_vec(),
            };
            commit(
                ledger.commit_step(
                    run,
                    &WorkDelta {
                        vocabulary: Some(saved),
                        ..Default::default()
                    },
                    &[(
                        at,
                        HistoryFact::EffectIntent {
                            kind: EffectKind::Publication,
                            reference: EffectReference::PublicationTarget(path.clone()),
                        },
                    )],
                ),
                sink,
                warnings,
            )?;
            let stores = FilesystemStores::open_read_only(&[])
                .map_err(|error| Box::new(crate::compose::errors::contained_error(error)))?;
            publication::repair(ledger, &stores, settings, sink, warnings)?;
        }
        Ok::<_, Box<ErrorDocument>>(added)
    })();
    let at = SystemClock.now();
    commit(
        ledger.commit_step(
            run,
            &WorkDelta::default(),
            &[
                (
                    at,
                    HistoryFact::WorkStep {
                        recording: None,
                        outcome: if result.is_ok() {
                            WorkOutcome::Completed
                        } else {
                            WorkOutcome::Failed
                        },
                    },
                ),
                (
                    at,
                    HistoryFact::RunFinished(if result.is_ok() {
                        RunOutcome::Completed
                    } else {
                        RunOutcome::Failed
                    }),
                ),
            ],
        ),
        sink,
        warnings,
    )?;
    let added = result?;
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: document(
            "confirm",
            serde_json::json!({"confirmed": {"term": true, "added": added, "path": path}}),
        ),
        human: format!("Confirmed term in {}\n", path.display()),
    })
}
