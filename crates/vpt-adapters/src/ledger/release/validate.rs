use super::*;
use vpt_application::ports::{NewRelease, RecordingRecord, ReleaseProvenance, ReleaseVersion};
use vpt_domain::{digest::Sha256Digest, time::UtcInstant};
pub(crate) const MAX_BYTES: usize = 16 * 1024 * 1024;
pub(crate) fn digest(bytes: &[u8]) -> Sha256Digest {
    crate::ledger::brief::validate::digest(bytes)
}
pub(crate) fn path(path: &std::path::Path) -> Result<&str, LedgerError> {
    vpt_domain::brief::document::source_path(path)
        .filter(|s| !s.chars().any(char::is_control))
        .ok_or_else(invalid)
}
pub(crate) fn instant(at: UtcInstant) -> Result<(), LedgerError> {
    crate::ledger::brief::validate::instant(at)
        .then_some(())
        .ok_or_else(invalid)
}
pub(crate) fn provenance(p: &ReleaseProvenance) -> Result<(), LedgerError> {
    instant(p.captured_at)?;
    if p.captured_offset.secs.unsigned_abs() >= 86_400
        || p.engines.is_empty()
        || p.engines.len() > 2
        || p.engines
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 4096 || s.chars().any(char::is_control))
        || p.open_lexical_flags > p.open_flags
        || p.open_flags > 65_536
        || p.diagnostics.len() > 3
        || p.diagnostics
            .iter()
            .enumerate()
            .any(|(i, d)| p.diagnostics[..i].contains(d))
        || p.reviewed && (p.open_flags != 0 || !p.diagnostics.is_empty())
    {
        return Err(invalid());
    }
    Ok(())
}
pub(crate) fn prepare(
    draft: &NewRelease,
    sequence: i64,
    recording: &RecordingRecord,
) -> Result<ReleaseRecord, LedgerError> {
    if sequence <= 0
        || draft.recording != recording.id
        || draft.provenance.captured_at != recording.captured_at
        || draft.provenance.captured_offset != recording.captured_offset
        || draft.text.len() > MAX_BYTES
        || draft.report.len() > MAX_BYTES
        || draft.text.contains('\0')
        || draft.report.contains('\0')
    {
        return Err(invalid());
    }
    path(&draft.report_root)?;
    let row = ReleaseRecord {
        sequence,
        recording: draft.recording.clone(),
        stage: draft.stage,
        destination: draft.destination.clone(),
        report_path: draft.report_root.join(format!(
            "{}.{}.{}.report.json",
            draft.recording,
            draft.stage.as_str(),
            sequence
        )),
        version: ReleaseVersion {
            content: digest(draft.text.as_bytes()),
            source: draft.source_digest,
            report: digest(draft.report.as_bytes()),
        },
        provenance: draft.provenance.clone(),
        created_at: draft.created_at,
        finalized_at: None,
    };
    record(&row)?;
    super::report::check(&draft.report, row.version)?;
    Ok(row)
}
pub(crate) fn record(row: &ReleaseRecord) -> Result<(), LedgerError> {
    let dest = path(&row.destination)?;
    path(&row.report_path)?;
    provenance(&row.provenance)?;
    instant(row.created_at)?;
    if let Some(at) = row.finalized_at {
        instant(at)?;
    }
    if row.sequence <= 0
        || !dest.ends_with(".md")
        || row.destination == row.report_path
        || row.report_path.file_name().and_then(|n| n.to_str())
            != Some(
                format!(
                    "{}.{}.{}.report.json",
                    row.recording,
                    row.stage.as_str(),
                    row.sequence
                )
                .as_str(),
            )
        || vpt_domain::identity::local_timestamp(
            row.provenance.captured_at,
            row.provenance.captured_offset,
        ) != row.recording.local_timestamp()
    {
        return Err(invalid());
    }
    Ok(())
}
