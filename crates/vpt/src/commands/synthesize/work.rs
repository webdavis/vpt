mod finish;
use super::{errors, observe, output, prepare, source};
use crate::{
    cli::output::Outcome,
    commands::note,
    compose::{errors::contained_error, history::commit, publication},
};
use std::collections::BTreeSet;
use vpt_adapters::{CommandAgent, FilesystemStores, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::history::RunId;
use vpt_protocol::error::ErrorDocument;

pub(crate) struct Writer<'a, E> {
    pub mentions: Option<&'a crate::compose::mentions::Context>,
    pub settings: &'a Settings,
    pub roots: &'a Roots,
    pub ledger: &'a SqliteLedger,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
}
impl<E: EventSink> Writer<'_, E> {
    pub(crate) fn run(
        &mut self,
        input: NoteInputs,
        run: RunId,
        committed: &mut bool,
    ) -> Result<Outcome, Box<ErrorDocument>> {
        let id = input.record.id.clone();
        let mut attempted = false;
        let mut accepted = false;
        let mut diagnostics = Vec::new();
        let result = (|| {
            self.roots.create_leaves().map_err(|error| {
                crate::compose::config_refusal::in_run(
                    self.ledger,
                    self.sink,
                    run,
                    error,
                    self.warnings,
                )
            })?;
            let stores = FilesystemStores::open(&[
                self.roots.stores.transcripts.clone(),
                self.roots.stores.analysis.clone(),
            ])
            .map_err(|error| Box::new(contained_error(error)))?;
            let loaded;
            let mentions = match self.mentions {
                Some(mentions) => mentions,
                None => {
                    loaded = crate::compose::mentions::Context::read(self.settings, self.roots)?;
                    &loaded
                }
            };
            note::write::Writer {
                mentions,
                ledger: self.ledger,
                stores: &stores,
                settings: self.settings,
                options: vpt_application::note::WriteOptions {
                    roots: &self.roots.stores,
                    analysis_content: None,
                    regenerate_expired: false,
                },
                sink: self.sink,
                warnings: self.warnings,
            }
            .write(input, run, committed)?;
            let mut input = observe::inputs(self.ledger, &id)?;
            finish::pending(self, run, &mut input)?;
            mentions.enrich(&mut input, None)?;
            let source = input
                .notes
                .iter()
                .find(|note| note.stage == NoteStage::Transcript)
                .ok_or_else(errors::invalid)?;
            let source_path = source.target.clone();
            let content = source::content(source)?;
            let digest = source.content_digest;
            source::check(&stores, source)?;
            attempted = true;
            crate::cli::progress::report(&id, "synthesize", "started", None);
            let generated = CommandAgent
                .synthesize(&SynthesisRequest {
                    settings: &self.settings.synthesis,
                    transcript: &source_path,
                    recording: &id,
                    language: &input
                        .review
                        .accepted
                        .as_ref()
                        .ok_or_else(errors::invalid)?
                        .requested_language,
                    content: &content,
                })
                .map_err(errors::command)?;
            diagnostics = generated.diagnostics.clone();
            source::check(&stores, source)?;
            if generated.input_digest != digest {
                return Err(errors::changed());
            }
            let at = SystemClock.now();
            let prepared = prepare::delta(
                input,
                generated,
                &prepare::Context {
                    settings: self.settings,
                    roots: self.roots,
                    stores: &stores,
                    ledger: self.ledger,
                    mentions,
                },
                at,
            )?;
            let facts = super::history::facts(&prepared, &id, at);
            let delta = prepared.delta;
            if let Err(error) = commit(
                self.ledger.commit_step(run, &delta, &facts),
                self.sink,
                self.warnings,
            ) {
                accepted = error.completed.contains(&id.to_string());
                *committed |= accepted;
                return Err(error);
            }
            accepted = true;
            *committed = true;
            publication::repair(
                self.ledger,
                &stores,
                self.settings,
                self.sink,
                self.warnings,
            )?;
            output::success(self.ledger, &id, &diagnostics)
        })();
        if attempted {
            let state = if accepted { "done" } else { "failed" };
            crate::cli::progress::report(&id, "synthesize", state, None);
        }
        let result = if result.is_err() && !accepted {
            finish::failure(self, run, &id, attempted).and(result)
        } else {
            result
        };
        result.map_err(|mut error| {
            error.diagnostics.extend(diagnostics);
            error
        })
    }
}
