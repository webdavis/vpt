use super::codec::{RECORDING_COLUMNS, SEEN_COLUMNS, stored};
use crate::ledger::sqlite::map;
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, RecordingRecord, SeenRow};
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;

pub(super) fn upsert_seen(connection: &Connection, row: &SeenRow) -> Result<(), LedgerError> {
    connection
        .execute(
            &format!(
                "INSERT INTO seen ({SEEN_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
                 ON CONFLICT(path) DO UPDATE SET file_name = excluded.file_name, size = excluded.size, \
                 mtime_secs = excluded.mtime_secs, mtime_nanos = excluded.mtime_nanos, flags = excluded.flags, \
                 last_seen = excluded.last_seen, deferral_count = excluded.deferral_count, \
                 deferral_reason = excluded.deferral_reason, deferred_size = excluded.deferred_size, \
                 source_gone_at = excluded.source_gone_at, recording = excluded.recording"
            ),
            params![
                row.path.to_string_lossy(),
                row.file_name,
                stored(row.size, "size")?,
                row.mtime.secs,
                i64::from(row.mtime.nanos),
                i64::from(row.flags),
                row.first_seen.secs,
                row.last_seen.secs,
                i64::from(row.deferral_count),
                row.deferral_reason.map(DeferralReason::as_str),
                row.deferred_size.map(|value| stored(value, "deferred size")).transpose()?,
                row.source_gone_at.map(|instant| instant.secs),
                row.recording.as_ref().map(RecordingId::as_str),
            ],
        )
        .map(|_| ())
        .map_err(map)
}

pub(super) fn insert_recording(
    connection: &Connection,
    recording: &RecordingRecord,
) -> Result<(), LedgerError> {
    connection
        .execute(
            &format!(
                "INSERT INTO recordings ({RECORDING_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
            ),
            params![
                recording.id.as_str(),
                recording.source_path.as_ref().map(|path| path.to_string_lossy().into_owned()),
                recording.digest.hex(),
                recording.captured_at.secs,
                recording.captured_offset.secs,
                stored(recording.duration_secs, "duration")?,
                recording.title,
                recording.title_source.as_str(),
                recording.ingested_at.secs,
                recording.audio_path.to_string_lossy(),
                recording.stages.transcribe.as_str(),
                recording.stages.note.as_str(),
                recording.stages.analysis.as_str(),
                recording.audio_trashed_at.map(|instant| instant.secs),
            ],
        )
        .map(|_| ())
        .map_err(map)
}
