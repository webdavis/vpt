use super::{
    Request,
    output::{self, Report, Untouched},
};
use crate::{
    cli::output::Outcome,
    compose::{
        errors::{ledger_error, open_error},
        publication, tag_notes, tag_vocabulary,
    },
};
use std::path::PathBuf;
use vpt_adapters::{
    Roots, SqliteLedger, known_tags,
    note_index::{Index, IndexedNote},
};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    identity::RecordingId,
    tags::{Rename, Tag},
};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Carrier {
    pub id: RecordingId,
    pub expected: Option<Vec<Tag>>,
    pub tags: Vec<Tag>,
}

pub(super) fn carriers(
    ledger: &SqliteLedger,
    rename: &Rename,
) -> Result<Vec<Carrier>, Box<ErrorDocument>> {
    let error = |error| Box::new(ledger_error(error));
    let mut found = Vec::new();
    for record in ledger.recordings().map_err(error)? {
        let expected = ledger.tags(&record.id).map_err(error)?;
        let notes = match expected {
            Some(_) => Vec::new(),
            None => Vec::from_iter(
                ledger
                    .note(&record.id, NoteStage::Transcript)
                    .map_err(error)?,
            ),
        };
        if let Some(tags) = rename.apply(&tag_notes::current(expected.as_deref(), &notes)) {
            found.push(Carrier {
                id: record.id,
                expected,
                tags,
            });
        }
    }
    found.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    Ok(found)
}

pub(super) fn untouched(
    roots: &Roots,
    index: &Index,
    rename: &Rename,
    planned: &[PathBuf],
) -> Result<Untouched, Box<ErrorDocument>> {
    let briefs = &roots.stores.briefs;
    let carries = |note: &&IndexedNote| note.metadata.tags.iter().any(|tag| rename.matches(tag));
    let brief_index = Index::read(std::slice::from_ref(briefs))
        .map_err(|error| Box::new(crate::compose::errors::store_error(error)))?;
    Ok(Untouched {
        briefs: brief_index.notes().iter().filter(carries).count(),
        notes: index
            .notes()
            .iter()
            .filter(|note| !planned.contains(&note.path) && !note.path.starts_with(briefs))
            .filter(carries)
            .count(),
    })
}

pub(super) fn preview(
    settings: &Settings,
    roots: &Roots,
    request: &Request<'_>,
) -> Result<Outcome, Box<ErrorDocument>> {
    let index = crate::compose::index::read(settings, roots)?;
    let (vocabulary, _) = tag_vocabulary::read(settings, &index)?;
    let rename = vocabulary
        .rename(request.old, request.new)
        .map_err(super::usage)?;
    let known = known_tags::prepare_renamed(&settings.tags.known_tags_path, &rename)
        .map_err(|error| publication::vocabulary::document(VocabularyKind::Tags, error))?;
    let mut recordings = Vec::new();
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|error| Box::new(open_error(error)))?;
    if let Some(ledger) = &ledger {
        for carrier in carriers(ledger, &rename)? {
            let inputs = ledger
                .note_inputs(&carrier.id)
                .map_err(|error| Box::new(ledger_error(error)))?
                .ok_or_else(crate::commands::note::errors::not_found)?;
            let notes = match inputs.has_renderable_notes() {
                true => inputs
                    .notes
                    .iter()
                    .map(|note| note.target.clone())
                    .collect(),
                false => Vec::new(),
            };
            recordings.push((carrier.id, notes));
        }
    }
    let planned: Vec<_> = recordings
        .iter()
        .flat_map(|(_, notes)| notes.clone())
        .collect();
    Ok(output::success(&Report {
        dry_run: true,
        recordings: &recordings,
        known_tags_changed: known.added(),
        untouched: untouched(roots, &index, &rename, &planned)?,
    }))
}
