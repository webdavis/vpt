use super::{
    errors,
    output::{self, Artifact},
    recording::provenance,
};
use crate::commands::redact::{prepare, source::Source};
use serde_json::Value;
use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    copy::{Identity, Kind, Provenance},
    identity::RecordingId,
    note::NoteStage,
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn copy(
    settings: &Settings,
    roots: &Roots,
    ledger: &SqliteLedger,
    id: &RecordingId,
    kind: Kind,
) -> Result<Value, Box<ErrorDocument>> {
    let stage = match kind {
        Kind::Transcript => NoteStage::Transcript,
        Kind::Analysis => NoteStage::Analysis,
        _ => return Err(errors::missing()),
    };
    let source = Source::read(ledger, roots, id, stage)?;
    let mut redactor = prepare::redactor::build(settings, roots)?;
    let masked = prepare::mask(&source, stage, settings, &mut redactor)?;
    let (engines, open_flags, reviewed) = provenance(&source.input, stage);
    let saved = source.input.notes.iter().find(|note| note.stage == stage);
    if ledger
        .pending_publications()
        .map_err(errors::ledger)?
        .iter()
        .any(|pending| saved.is_some_and(|note| note.target == pending.target))
    {
        return Err(errors::pending());
    }
    let record = &source.input.record;
    let mut value = output::value(
        roots,
        Artifact {
            kind,
            identity: Identity::Recording(id),
            path: &source.path,
            digest: source.note.digest,
            body: &labelled(settings, &masked.content.text),
            provenance: Provenance {
                captured: Some((record.captured_at, record.captured_offset)),
                occasion_at: None,
                engines: &engines,
                open_flags,
                reviewed: reviewed && saved.is_some_and(|note| note.digest == source.note.digest),
            },
            withhold_path: true,
        },
    )?;
    let emitted = value["content"].as_str().unwrap_or_default().as_bytes();
    value["content_sha256"] = source.stores.digest_bytes(emitted).hex().into();
    Ok(value)
}

fn labelled(settings: &Settings, text: &str) -> String {
    match settings.share.source_line.as_str() {
        "" => text.to_owned(),
        line => format!("{line}\n\n{text}"),
    }
}
