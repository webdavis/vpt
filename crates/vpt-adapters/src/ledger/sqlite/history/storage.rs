use super::{codec, map};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, ExportFailureId, LedgerNamespace, RunId};
use vpt_domain::run::{ExportCursor, ExportPosition, FailureWindow, HistoryFact, HistoryRecord};
use vpt_domain::time::UtcInstant;

pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("history state mismatch".into())
}

pub(super) fn corrupt() -> LedgerError {
    LedgerError::Corrupt("unreadable history column".into())
}

pub(super) fn column(error: rusqlite::Error) -> LedgerError {
    match error {
        rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::IntegralValueOutOfRange(..)
        | rusqlite::Error::Utf8Error(..) => corrupt(),
        other => map(other),
    }
}

pub(super) fn positive(value: i64) -> Result<u64, LedgerError> {
    let value = u64::try_from(value).map_err(|_| corrupt())?;
    (value > 0).then_some(value).ok_or_else(corrupt)
}

pub(super) fn stored(value: u64) -> Result<i64, LedgerError> {
    i64::try_from(value)
        .map_err(|_| LedgerError::Corrupt("history sequence exceeds database range".into()))
}

pub(super) fn parse_unsigned(text: &str) -> Result<u64, LedgerError> {
    let parsed = text.parse::<u64>().map_err(|_| corrupt())?;
    (parsed.to_string() == text)
        .then_some(parsed)
        .ok_or_else(corrupt)
}

pub(super) fn namespace(connection: &Connection) -> Result<LedgerNamespace, LedgerError> {
    let bytes: Vec<u8> = connection
        .query_row(
            "SELECT namespace FROM history_identity WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(column)?;
    let bytes: [u8; 16] = bytes.try_into().map_err(|_| corrupt())?;
    Ok(LedgerNamespace::new(bytes))
}

pub(super) fn cursor(connection: &Connection) -> Result<Option<ExportCursor>, LedgerError> {
    let fields: (Option<i64>, Option<String>, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT cursor_generation, cursor_device, cursor_inode, cursor_end FROM history_identity WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(column)?;
    match fields {
        (None, None, None, None) => Ok(None),
        (Some(generation), Some(device), Some(inode), Some(end)) => Ok(Some(ExportCursor {
            generation: u64::try_from(generation).map_err(|_| corrupt())?,
            position: ExportPosition {
                device: parse_unsigned(&device)?,
                inode: parse_unsigned(&inode)?,
                end_offset: parse_unsigned(&end)?,
            },
        })),
        _ => Err(corrupt()),
    }
}

pub(super) fn save_cursor(
    transaction: &Transaction<'_>,
    cursor: ExportCursor,
) -> Result<(), LedgerError> {
    transaction.execute(
        "UPDATE history_identity SET cursor_generation = ?1, cursor_device = ?2, cursor_inode = ?3, cursor_end = ?4 WHERE id = 1",
        params![stored(cursor.generation)?, cursor.position.device.to_string(), cursor.position.inode.to_string(), cursor.position.end_offset.to_string()],
    ).map_err(map)?;
    Ok(())
}

pub(super) fn summarized_through(
    connection: &Connection,
    records: &[HistoryRecord],
) -> Result<Option<ExportFailureId>, LedgerError> {
    let namespace = namespace(connection)?;
    let mut current = None;
    for record in records {
        if let HistoryFact::RecoverySummary {
            window: Some(window),
            ..
        } = &record.fact
        {
            if window.after != current
                || window.through.namespace() != namespace
                || window.after.is_some_and(|after: ExportFailureId| {
                    after.namespace() != namespace || after.sequence() >= window.through.sequence()
                })
                || !failure_exists(connection, window.through)?
            {
                return Err(corrupt());
            }
            current = Some(window.through);
        }
    }
    Ok(current)
}

fn failure_exists(connection: &Connection, id: ExportFailureId) -> Result<bool, LedgerError> {
    let found: Option<i64> = connection
        .query_row(
            "SELECT sequence FROM history_export_failures WHERE sequence = ?1",
            [stored(id.sequence())?],
            |row| row.get(0),
        )
        .optional()
        .map_err(column)?;
    Ok(found.is_some())
}

fn validate_window(
    transaction: &Transaction<'_>,
    window: FailureWindow,
) -> Result<(), LedgerError> {
    let records = records(transaction, false)?;
    let current = summarized_through(transaction, &records)?;
    let namespace = namespace(transaction)?;
    if window.after != current
        || window.through.namespace() != namespace
        || window.after.is_some_and(|after| {
            after.namespace() != namespace || after.sequence() >= window.through.sequence()
        })
        || !failure_exists(transaction, window.through)?
    {
        return Err(conflict());
    }
    Ok(())
}

pub(super) fn record(
    transaction: &Transaction<'_>,
    run: RunId,
    at: UtcInstant,
    fact: HistoryFact,
) -> Result<HistoryRecord, LedgerError> {
    if run.namespace() != namespace(transaction)? {
        return Err(conflict());
    }
    if let HistoryFact::Event(event) = &fact
        && event.occurred_at != at
    {
        return Err(conflict());
    }
    if let HistoryFact::RecoveryProgress { baseline, .. } = &fact
        && !baseline.is_valid_for(run.namespace())
    {
        return Err(conflict());
    }
    if let HistoryFact::EffectObserved { intent, .. } = &fact {
        if intent.namespace() != run.namespace() {
            return Err(conflict());
        }
        let text: Option<String> = transaction
            .query_row(
                "SELECT fact FROM history_records WHERE sequence = ?1",
                [stored(intent.sequence())?],
                |row| row.get(0),
            )
            .optional()
            .map_err(column)?;
        if !matches!(
            text.as_deref().map(codec::decode).transpose()?,
            Some(HistoryFact::EffectIntent { .. })
        ) {
            return Err(conflict());
        }
    }
    if let HistoryFact::HookFailed { event, .. } = &fact {
        if event.namespace() != run.namespace() {
            return Err(conflict());
        }
        let referenced: Option<(i64, String)> = transaction
            .query_row(
                "SELECT run_sequence, fact FROM history_records WHERE sequence = ?1",
                [stored(event.sequence())?],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(column)?;
        if !matches!(referenced, Some((sequence, text)) if positive(sequence)? == run.sequence() && matches!(codec::decode(&text)?, HistoryFact::Event(_)))
        {
            return Err(conflict());
        }
    }
    if let HistoryFact::RecoverySummary {
        window: Some(window),
        ..
    } = &fact
    {
        validate_window(transaction, *window)?;
    }
    let stored_fact = codec::encode(&fact)?;
    transaction
        .execute(
            "INSERT INTO history_records (run_sequence, occurred_at, fact) VALUES (?1, ?2, ?3)",
            params![stored(run.sequence())?, at.secs, stored_fact],
        )
        .map_err(map)?;
    let sequence = positive(transaction.last_insert_rowid())?;
    Ok(HistoryRecord {
        id: EventId::new(run.namespace(), sequence),
        run,
        occurred_at: at,
        fact,
        appended_at: None,
        export_position: None,
    })
}

mod read;
pub(super) use read::records;
