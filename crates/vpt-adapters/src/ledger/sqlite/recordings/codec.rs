use rusqlite::{Row, types::FromSql};
use std::path::PathBuf;
use vpt_application::ports::{
    LedgerError, RecordingRecord, SeenRow, StageState, StageStates, TitleOrigin,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

pub(super) const SEEN_COLUMNS: &str = "path, file_name, size, mtime_secs, mtime_nanos, flags, first_seen, last_seen, deferral_count, deferral_reason, deferred_size, source_gone_at, recording";
pub(super) const RECORDING_COLUMNS: &str = "id, source_path, digest, captured_at, captured_offset, duration_secs, title, title_source, ingested_at, audio_path, stage_transcribe, stage_note, stage_synthesis, audio_trashed_at";

fn corrupt(field: &str) -> LedgerError {
    LedgerError::Corrupt(format!("unreadable {field}"))
}

fn field<T: FromSql>(row: &Row<'_>, index: usize, name: &str) -> Result<T, LedgerError> {
    row.get(index).map_err(|_| corrupt(name))
}

fn unsigned(value: i64, name: &str) -> Result<u64, LedgerError> {
    u64::try_from(value).map_err(|_| corrupt(name))
}

fn narrow(value: i64, name: &str) -> Result<u32, LedgerError> {
    u32::try_from(value).map_err(|_| corrupt(name))
}

pub(super) fn stored(value: u64, name: &str) -> Result<i64, LedgerError> {
    i64::try_from(value).map_err(|_| corrupt(name))
}

pub(super) fn seen_from_row(row: &Row<'_>) -> Result<SeenRow, LedgerError> {
    let nanos = narrow(field(row, 4, "mtime nanoseconds")?, "mtime nanoseconds")?;
    if nanos >= 1_000_000_000 {
        return Err(corrupt("mtime nanoseconds"));
    }
    let reason: Option<String> = field(row, 9, "deferral reason")?;
    let recording: Option<String> = field(row, 12, "recording id")?;
    Ok(SeenRow {
        path: PathBuf::from(field::<String>(row, 0, "seen path")?),
        file_name: field(row, 1, "file name")?,
        size: unsigned(field(row, 2, "size")?, "size")?,
        mtime: FileTime {
            secs: field(row, 3, "mtime")?,
            nanos,
        },
        flags: narrow(field(row, 5, "flags")?, "flags")?,
        first_seen: UtcInstant {
            secs: field(row, 6, "first seen")?,
        },
        last_seen: UtcInstant {
            secs: field(row, 7, "last seen")?,
        },
        deferral_count: narrow(field(row, 8, "deferral count")?, "deferral count")?,
        deferral_reason: reason
            .map(|value| DeferralReason::parse(&value).ok_or_else(|| corrupt("deferral reason")))
            .transpose()?,
        deferred_size: field::<Option<i64>>(row, 10, "deferred size")?
            .map(|value| unsigned(value, "deferred size"))
            .transpose()?,
        source_gone_at: field::<Option<i64>>(row, 11, "source gone")?
            .map(|secs| UtcInstant { secs }),
        recording: recording
            .map(|value| RecordingId::parse(&value).map_err(|_| corrupt("recording id")))
            .transpose()?,
    })
}

pub(super) fn record_from_row(row: &Row<'_>) -> Result<RecordingRecord, LedgerError> {
    let stage = |index| -> Result<StageState, LedgerError> {
        let value: String = field(row, index, "stage state")?;
        StageState::parse(&value).ok_or_else(|| corrupt("stage state"))
    };
    let digest: String = field(row, 2, "digest")?;
    let offset: i64 = field(row, 4, "captured offset")?;
    let offset = i32::try_from(offset).map_err(|_| corrupt("captured offset"))?;
    if offset.unsigned_abs() >= 86_400 {
        return Err(corrupt("captured offset"));
    }
    let origin: String = field(row, 7, "title source")?;
    Ok(RecordingRecord {
        id: RecordingId::parse(&field::<String>(row, 0, "recording id")?)
            .map_err(|_| corrupt("recording id"))?,
        source_path: field::<Option<String>>(row, 1, "source path")?.map(PathBuf::from),
        digest: Sha256Digest::from_hex(&digest).ok_or_else(|| corrupt("digest"))?,
        captured_at: UtcInstant {
            secs: field(row, 3, "capture time")?,
        },
        captured_offset: UtcOffset { secs: offset },
        duration_secs: unsigned(field(row, 5, "duration")?, "duration")?,
        title: field(row, 6, "title")?,
        title_source: TitleOrigin::parse(&origin).ok_or_else(|| corrupt("title source"))?,
        ingested_at: UtcInstant {
            secs: field(row, 8, "ingest time")?,
        },
        audio_path: PathBuf::from(field::<String>(row, 9, "audio path")?),
        stages: StageStates {
            transcribe: stage(10)?,
            note: stage(11)?,
            synthesis: stage(12)?,
        },
        audio_trashed_at: field::<Option<i64>>(row, 13, "audio trashed")?
            .map(|secs| UtcInstant { secs }),
    })
}
