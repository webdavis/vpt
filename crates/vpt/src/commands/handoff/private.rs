use super::{
    errors, file,
    output::{self, Artifact},
};
use crate::cli::output::Outcome;
use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    handoff::{Identity, Kind, Provenance},
    identity::RecordingId,
    review::{FlagContent, Resolution},
};
use vpt_protocol::error::ErrorDocument;
pub(super) fn run(
    ledger: &SqliteLedger,
    roots: &Roots,
    id: &RecordingId,
    kind: Kind,
) -> Result<Outcome, Box<ErrorDocument>> {
    let stage = match kind {
        Kind::Transcript => NoteStage::Transcript,
        Kind::Analysis => NoteStage::Analysis,
        _ => return Err(errors::missing()),
    };
    let input = ledger
        .note_inputs(id)
        .map_err(errors::ledger)?
        .ok_or_else(errors::missing)?;
    let Some(saved) = input.notes.iter().find(|n| n.stage == stage) else {
        return Err(missing_stages(ledger, id)?);
    };
    if ledger
        .pending_publications()
        .map_err(errors::ledger)?
        .iter()
        .any(|p| p.target == saved.target)
    {
        return Err(errors::pending());
    }
    let root = match stage {
        NoteStage::Transcript => &roots.stores.transcripts,
        NoteStage::Analysis => &roots.stores.analysis,
    };
    let Some(note) = file::read(&saved.target, root)? else {
        return Err(missing_stages(ledger, id)?);
    };
    let document =
        vpt_adapters::note_document::Document::parse(&note.text).map_err(|_| errors::changed())?;
    document
        .check_owner(id, stage.as_str())
        .map_err(|_| errors::changed())?;
    let (engines, flags, reviewed) = provenance(&input, stage);
    output::success(
        roots,
        Artifact {
            kind,
            identity: Identity::Recording(id),
            path: &saved.target,
            digest: note.digest,
            body: file::body(&note.text)?,
            provenance: Provenance {
                captured: Some((input.record.captured_at, input.record.captured_offset)),
                occasion_at: None,
                engines: &engines,
                open_flags: flags,
                reviewed: reviewed && note.digest == saved.digest,
            },
        },
    )
}
pub(super) fn provenance(input: &NoteInputs, stage: NoteStage) -> (Vec<String>, u32, bool) {
    let mut flags = 0u32;
    let mut diagnostic = false;
    for flag in input
        .review
        .flags
        .iter()
        .filter(|f| f.resolution == Resolution::Open)
    {
        match &flag.content {
            FlagContent::Lexical(flag) if flag.class.surfaced() => flags = flags.saturating_add(1),
            FlagContent::Verification(_) => flags = flags.saturating_add(1),
            FlagContent::Diagnostic(_) => diagnostic = true,
            _ => (),
        }
    }
    let Some(accepted) = &input.review.accepted else {
        return (vec![], flags, false);
    };
    let engines = std::iter::once(&accepted.transcript.engine)
        .chain(accepted.checker_engine.as_ref())
        .map(|e| format!("{}:{}", e.name, e.model))
        .collect();
    (
        engines,
        flags,
        accepted.lexical_classification
            && flags == 0
            && !diagnostic
            && !input.review.stale_artifacts.contains(&stage),
    )
}
pub(super) fn missing_stages(
    ledger: &SqliteLedger,
    id: &RecordingId,
) -> Result<Box<ErrorDocument>, Box<ErrorDocument>> {
    let input = ledger.note_inputs(id).map_err(errors::ledger)?;
    let mut stages = Vec::new();
    if let Some(input) = input {
        for note in input.notes {
            if file::exists(&note.target)? {
                stages.push(note.stage.as_str());
            }
        }
    }
    for sequence in ledger.releases(id).map_err(errors::ledger)? {
        if let Some(release) = ledger.release(sequence).map_err(errors::ledger)?
            && file::exists(&release.destination)?
        {
            stages.push("released");
            break;
        }
    }
    let mut error = errors::missing();
    error.message = format!(
        "no existing artifact for this identity and stage; available stages: {}",
        if stages.is_empty() {
            "none".into()
        } else {
            stages.join(", ")
        }
    );
    Ok(error)
}
