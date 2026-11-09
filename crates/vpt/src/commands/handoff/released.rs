use super::{
    errors, file,
    output::{self, Artifact},
};
use crate::cli::output::Outcome;
use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::ports::ReleaseLedger;
use vpt_domain::{
    handoff::{Identity, Kind, Provenance},
    identity::RecordingId,
};
use vpt_protocol::error::ErrorDocument;
pub(super) fn run(
    ledger: &SqliteLedger,
    roots: &Roots,
    id: &RecordingId,
) -> Result<Outcome, Box<ErrorDocument>> {
    for sequence in ledger.releases(id).map_err(errors::ledger)? {
        let release = ledger
            .release(sequence)
            .map_err(errors::ledger)?
            .ok_or_else(errors::missing)?;
        let parent = release.destination.parent().ok_or_else(errors::changed)?;
        let Some(note) = file::version(&release.destination, parent, release.version.content)?
        else {
            continue;
        };
        if note.digest != release.version.content {
            return Err(errors::changed());
        }
        let p = &release.provenance;
        return output::success(
            roots,
            Artifact {
                kind: Kind::Released,
                identity: Identity::Recording(id),
                path: &release.destination,
                digest: note.digest,
                body: file::body(&note.text)?,
                provenance: Provenance {
                    captured: Some((p.captured_at, p.captured_offset)),
                    occasion_at: None,
                    engines: &p.engines,
                    open_flags: p.open_flags,
                    reviewed: p.reviewed,
                },
            },
        );
    }
    for sequence in ledger.pending_releases().map_err(errors::ledger)? {
        if ledger
            .release(sequence)
            .map_err(errors::ledger)?
            .is_some_and(|r| r.recording == *id)
        {
            return Err(errors::pending());
        }
    }
    Err(super::private::missing_stages(ledger, id)?)
}
