use super::{errors, history};
use crate::compose::{history::commit, publication};
use std::{collections::BTreeSet, path::PathBuf};
use vpt_adapters::{FilesystemStores, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::history::RunId;
use vpt_protocol::error::ErrorDocument;

pub(crate) struct Writer<'a, E> {
    pub mentions: &'a crate::compose::mentions::Context,
    pub ledger: &'a SqliteLedger,
    pub stores: &'a FilesystemStores,
    pub settings: &'a Settings,
    pub options: vpt_application::note::WriteOptions<'a>,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
}

impl<E: EventSink> Writer<'_, E> {
    pub(crate) fn write(
        &mut self,
        mut inputs: NoteInputs,
        run: RunId,
        committed: &mut bool,
    ) -> Result<PathBuf, Box<ErrorDocument>> {
        self.mentions.enrich(&mut inputs, None)?;
        let at = SystemClock.now();
        let delta = vpt_application::note::prepare(
            &inputs,
            self.settings,
            self.options,
            at,
            SystemClock.offset_at(at),
            self.stores,
            &vpt_adapters::note_document::NoteFormatter,
        )
        .map_err(errors::document)?;
        let target = delta.notes[0].artifact.target.clone();
        if let Err(error) = commit(
            self.ledger
                .commit_step(run, &delta, &history::facts(&delta, &inputs.record.id, at)),
            self.sink,
            self.warnings,
        ) {
            *committed |= error.completed.contains(&inputs.record.id.to_string());
            return Err(error);
        }
        *committed = true;
        publication::repair(
            self.ledger,
            self.stores,
            self.settings,
            self.sink,
            self.warnings,
        )
        .map_err(|mut error| {
            let id = inputs.record.id.to_string();
            if !error.completed.contains(&id) {
                error.completed.push(id);
            }
            error
        })?;
        Ok(target)
    }
}
