use super::{check, errors, inspect, output};
use crate::{
    cli::output::Outcome,
    compose::{errors::ledger_error, history::commit},
};
use std::{collections::BTreeSet, path::Path};
use vpt_adapters::{SqliteLedger, SystemClock, note_document::Document};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    history::RunId,
    review::{FlagContent, Resolution},
    run::{HistoryFact, WorkOutcome},
};
use vpt_protocol::error::ErrorDocument;

pub(crate) struct Writer<'a, E> {
    pub settings: &'a Settings,
    pub roots: &'a vpt_adapters::Roots,
    pub ledger: &'a SqliteLedger,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
}

impl<E: EventSink> Writer<'_, E> {
    pub fn run(
        &mut self,
        path: &Path,
        input: NoteInputs,
        run: RunId,
        committed: &mut bool,
    ) -> Result<Outcome, Box<ErrorDocument>> {
        let settings = self.settings;
        let ledger = self.ledger;
        let sink = &mut *self.sink;
        let warnings = &mut *self.warnings;
        let source = inspect::read(path, &input, self.roots)?;
        let stage = source.stage.ok_or_else(errors::ownership)?;
        let checked = check::run(&source, &input, settings.analysis.grounding_window_secs)?;
        let previous = input
            .notes
            .iter()
            .find(|note| note.stage == stage)
            .ok_or_else(errors::ownership)?;
        let basis = vpt_adapters::grounding_basis_for(
            input.review.accepted.as_ref().ok_or_else(errors::content)?,
            &input.review.flags,
        )
        .map_err(|error| Box::new(ledger_error(error)))?;
        let retained_open = input
            .review
            .flags
            .iter()
            .filter(|flag| {
                flag.resolution == Resolution::Open
                    && match &flag.content {
                        FlagContent::Lexical(flag) => flag.class.surfaced(),
                        FlagContent::Verification(flag) => flag.artifact != stage,
                        FlagContent::Diagnostic(_) => false,
                    }
            })
            .count();
        let parsed = Document::parse(&source.note.text).map_err(|_| errors::content())?;
        let next = parsed
            .replace_content(&checked.content)
            .map_err(|_| errors::content())?;
        let next = Document::parse(&next)
            .map_err(|_| errors::content())?
            .with_open_flags(retained_open + checked.open)
            .map_err(|_| errors::content())?;
        let next_line = Document::parse(&next)
            .map_err(|_| errors::content())?
            .content_start_line();
        let mut report = checked.report;
        let previous_line = parsed.content_start_line();
        for finding in &mut report {
            finding.line = finding.line - previous_line + next_line;
        }
        let mut artifact = previous.clone();
        artifact
            .managed_tags
            .retain(|tag| parsed.tags.contains(tag));
        artifact.text = next;
        artifact.digest = source.stores.digest_bytes(artifact.text.as_bytes());
        artifact.content_digest = source.stores.digest_bytes(checked.content.as_bytes());
        let at = SystemClock.now();
        artifact.updated_at = at;
        let delta = WorkDelta {
            verification: Some(VerificationChange {
                grounding_window_secs: settings.analysis.grounding_window_secs,
                recording: input.record.id.clone(),
                artifact: stage,
                input_digest: checked.input_digest,
                basis,
                findings: checked.findings,
                at,
                observed: Some(source.note.clone()),
            }),
            commit: LedgerCommit {
                publications: vec![DirtyPublication {
                    target: artifact.target.clone(),
                    expected_previous: Some(source.note.digest),
                    intended: artifact.digest,
                    recorded_at: at,
                }],
                ..Default::default()
            },
            notes: vec![NoteChange {
                artifact,
                expected_saved: Some(previous.digest),
                relocated_from: None,
            }],
            ..Default::default()
        };
        let facts = [(
            at,
            HistoryFact::WorkStep {
                recording: Some(input.record.id.clone()),
                outcome: WorkOutcome::Completed,
            },
        )];
        if let Err(error) = commit(ledger.commit_step(run, &delta, &facts), sink, warnings) {
            *committed = error.completed.contains(&input.record.id.to_string());
            return Err(error);
        }
        *committed = true;
        crate::compose::publication::repair(ledger, &source.stores, settings, sink, warnings)?;
        Ok(output::success(path, true, &report))
    }
}
