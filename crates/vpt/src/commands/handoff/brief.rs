use super::{
    errors, file,
    output::{self, Artifact},
};
use crate::cli::output::Outcome;
use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    handoff::{Identity, Kind, Provenance},
    occasion::OccasionId,
};
use vpt_protocol::error::ErrorDocument;
pub(super) fn run(
    ledger: &SqliteLedger,
    roots: &Roots,
    id: &OccasionId,
) -> Result<Outcome, Box<ErrorDocument>> {
    let occasion = ledger
        .occasion(id)
        .map_err(errors::ledger)?
        .ok_or_else(errors::missing)?;
    let saved = ledger
        .brief(id)
        .map_err(errors::ledger)?
        .ok_or_else(errors::missing)?;
    if occasion.brief_path.as_ref() != Some(&saved.target) {
        return Err(errors::changed());
    }
    if saved.published_at.is_none()
        || ledger
            .pending_publications()
            .map_err(errors::ledger)?
            .iter()
            .any(|p| p.target == saved.target)
    {
        return Err(errors::pending());
    }
    let note = file::read(&saved.target, &roots.stores.briefs)?.ok_or_else(errors::missing)?;
    let document = vpt_adapters::note_document::brief::Document::parse(&note.text)
        .map_err(|_| errors::changed())?;
    document.check_owner(id).map_err(|_| errors::changed())?;
    let mut sources = Vec::new();
    let mut engines = std::collections::BTreeSet::new();
    let capacity = || errors::document(vpt_domain::handoff::Error::CapacityExceeded);
    if saved.snapshot.pack.sources.len() > 65_536 {
        return Err(capacity());
    }
    let mut engine_bytes = 1024 * 1024usize;
    let mut reviewed =
        saved.snapshot.pack.unresolved.reviewed && note.digest == saved.version.content;
    for source in &saved.snapshot.pack.sources {
        let input = ledger
            .note_inputs(&source.recording)
            .map_err(errors::ledger)?
            .ok_or_else(errors::missing)?;
        let (names, _, current_reviewed) =
            super::private::provenance(&input, NoteStage::Transcript);
        reviewed &= current_reviewed
            && input
                .notes
                .iter()
                .any(|n| n.stage == NoteStage::Transcript && n.digest == source.digest);
        for name in names {
            if !engines.contains(&name) {
                engine_bytes = engine_bytes.checked_sub(name.len()).ok_or_else(capacity)?;
                engines.insert(name);
            }
        }
        sources.push(source.recording.clone());
    }
    let engines: Vec<_> = engines.into_iter().collect();
    let flags = u32::try_from(saved.snapshot.pack.unresolved.flags)
        .map_err(|_| errors::document(vpt_domain::handoff::Error::CapacityExceeded))?;
    output::success(
        roots,
        Artifact {
            kind: Kind::Brief,
            identity: Identity::Occasion {
                id,
                sources: &sources,
            },
            path: &saved.target,
            digest: note.digest,
            body: file::body(&note.text)?,
            provenance: Provenance {
                captured: None,
                occasion_at: Some(&saved.snapshot.occasion.at),
                engines: &engines,
                open_flags: flags,
                reviewed,
            },
        },
    )
}
