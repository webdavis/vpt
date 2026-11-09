use super::{errors, output::Written, prepare, source::Source};
use crate::{
    cli::args::redact::Options,
    compose::{self, history::commit},
};
use std::{collections::BTreeSet, path::Path};
use vpt_adapters::{ReleaseFiles, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::history::RunId;
use vpt_protocol::error::ErrorDocument;
pub(crate) struct Writer<'a, E> {
    pub ledger: &'a SqliteLedger,
    pub settings: &'a Settings,
    pub roots: &'a Roots,
    pub options: &'a Options,
    pub destination: &'a Path,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
}
impl<E: EventSink> Writer<'_, E> {
    pub fn write(
        &mut self,
        run: RunId,
        completed: &mut bool,
        automatic: bool,
    ) -> Result<Option<Written>, Box<ErrorDocument>> {
        let source = Source::read(
            self.ledger,
            self.roots,
            &self.options.id,
            self.options.stage,
        )?;
        let prepared = prepare::run(&source, self.options, self.settings, self.roots)?;
        self.roots.create_leaves().map_err(|error| {
            compose::config_refusal::in_run(self.ledger, self.sink, run, error, self.warnings)
        })?;
        let files = ReleaseFiles::open(self.roots, self.destination)
            .map_err(compose::releases::errors::document)?;
        if automatic
            && super::unchanged::matches(
                self.ledger,
                &self.options.id,
                self.options.stage,
                files.destination(),
                source.stores.digest_bytes(prepared.content.text.as_bytes()),
            )?
        {
            return Ok(None);
        }
        let at = SystemClock.now();
        let target = files
            .select(
                &self.options.id,
                self.options.stage,
                prepared.content.text.as_bytes(),
                at,
                SystemClock.offset_at(at),
            )
            .map_err(compose::releases::errors::document)?;
        source.unchanged()?;
        let draft = prepared.draft(&source, self.options, self.roots, target.path, at);
        let saved = match self.ledger.prepare_release(run, &draft) {
            HistoryWrite::Committed(saved) => saved,
            HistoryWrite::CommittedWithFailure { error, .. } | HistoryWrite::Unconfirmed(error) => {
                return Err(errors::ledger(error));
            }
        };
        commit(
            HistoryWrite::Committed(saved.receipt),
            self.sink,
            self.warnings,
        )?;
        let payload = self
            .ledger
            .release_payload(saved.release.sequence)
            .map_err(errors::ledger)?
            .ok_or_else(|| {
                errors::ledger(LedgerError::Corrupt("release payload missing".into()))
            })?;
        let version = files
            .publish(&payload)
            .map_err(compose::releases::errors::document)?;
        commit(
            self.ledger
                .complete_release(run, saved.release.sequence, version, SystemClock.now()),
            self.sink,
            self.warnings,
        )?;
        *completed = true;
        Ok(Some(Written {
            release: saved.release,
            summary: prepared.summary,
            collision: target.collision,
        }))
    }
}
