use super::{SqliteLedger, map, storage};
use rusqlite::{Connection, OptionalExtension, params};
use vpt_application::ports::{HistoryWrite, LedgerError};
use vpt_domain::history::EventId;
use vpt_domain::run::{ExportCursor, ExportFailure, ExportPosition, RecoveryView};
use vpt_domain::time::UtcInstant;

mod support;

pub(super) fn view(ledger: &SqliteLedger) -> Result<RecoveryView, LedgerError> {
    ledger.read(|connection| {
        let records = storage::records(connection, false)?;
        let summarized_through = storage::summarized_through(connection, &records)?;
        Ok(RecoveryView {
            records,
            failures: support::failures(connection)?,
            restarts: support::restarts(connection)?,
            active_cursor: storage::cursor(connection)?,
            summarized_through,
        })
    })
}

pub(super) fn acknowledge(
    ledger: &SqliteLedger,
    event: EventId,
    expected: Option<ExportCursor>,
    position: ExportPosition,
    appended_at: UtcInstant,
) -> HistoryWrite<()> {
    ledger.history_write(|transaction| {
        if event.namespace() != storage::namespace(transaction)? || storage::cursor(transaction)? != expected {
            return Err(storage::conflict());
        }
        if let Some(cursor) = expected {
            if position.device != cursor.position.device
                || position.inode != cursor.position.inode
                || position.end_offset <= cursor.position.end_offset
            {
                return Err(storage::conflict());
            }
        } else if position.end_offset == 0 {
            return Err(storage::conflict());
        }
        let first: Option<i64> = transaction.query_row(
            "SELECT sequence FROM history_records WHERE appended_at IS NULL ORDER BY sequence LIMIT 1",
            [],
            |row| row.get(0),
        ).optional().map_err(storage::column)?;
        if first.map(storage::positive).transpose()? != Some(event.sequence()) {
            return Err(storage::conflict());
        }
        let changed = transaction.execute(
            "UPDATE history_records SET appended_at = ?1, export_device = ?2, export_inode = ?3, export_end = ?4 WHERE sequence = ?5 AND appended_at IS NULL",
            params![appended_at.secs, position.device.to_string(), position.inode.to_string(), position.end_offset.to_string(), storage::stored(event.sequence())?],
        ).map_err(map)?;
        if changed != 1 {
            return Err(storage::conflict());
        }
        storage::save_cursor(transaction, ExportCursor { generation: expected.map_or(0, |cursor| cursor.generation), position })
    })
}

pub(super) fn note_failure(ledger: &SqliteLedger, failure: &ExportFailure) -> HistoryWrite<()> {
    ledger.history_write(|transaction| {
        if failure.event.namespace() != storage::namespace(transaction)?
            || failure.last_confirmed != latest_acknowledged_position(transaction)?
        {
            return Err(storage::conflict());
        }
        let pending: Option<i64> = transaction.query_row(
            "SELECT sequence FROM history_records WHERE sequence = ?1 AND appended_at IS NULL",
            [storage::stored(failure.event.sequence())?],
            |row| row.get(0),
        ).optional().map_err(storage::column)?;
        if pending.is_none() {
            return Err(storage::conflict());
        }
        let position = failure.last_confirmed;
        transaction.execute(
            "INSERT INTO history_export_failures (event_sequence, observed_at, class, last_device, last_inode, last_end, observed_file_json, partial_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                storage::stored(failure.event.sequence())?,
                failure.observed_at.secs,
                support::class_tag(failure.class),
                position.map(|value| value.device.to_string()),
                position.map(|value| value.inode.to_string()),
                position.map(|value| value.end_offset.to_string()),
                failure.observed_file.map(support::encode_observed_file),
                failure.partial.map(support::encode_partial),
            ],
        ).map_err(map)?;
        Ok(())
    })
}

fn latest_acknowledged_position(
    connection: &Connection,
) -> Result<Option<ExportPosition>, LedgerError> {
    let position: Option<(String, String, String)> = connection.query_row(
        "SELECT export_device, export_inode, export_end FROM history_records WHERE appended_at IS NOT NULL ORDER BY sequence DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(storage::column)?;
    position
        .map(|(device, inode, end)| {
            Ok(ExportPosition {
                device: storage::parse_unsigned(&device)?,
                inode: storage::parse_unsigned(&inode)?,
                end_offset: storage::parse_unsigned(&end)?,
            })
        })
        .transpose()
}

pub(super) fn restart(
    ledger: &SqliteLedger,
    expected: ExportCursor,
    observed: ExportPosition,
    observed_at: UtcInstant,
) -> HistoryWrite<()> {
    ledger.history_write(|transaction| {
        if storage::cursor(transaction)? != Some(expected) {
            if support::restarts(transaction)?.iter().any(|restart| {
                restart.old == expected && restart.new.position == observed
            }) {
                return Ok(());
            }
            return Err(storage::conflict());
        }
        if observed.device == expected.position.device
            && observed.inode == expected.position.inode
            && observed.end_offset >= expected.position.end_offset
        {
            return Err(storage::conflict());
        }
        let generation = expected.generation.checked_add(1).ok_or_else(storage::conflict)?;
        let new = ExportCursor { generation, position: observed };
        transaction.execute(
            "INSERT INTO history_export_restarts (old_generation, old_device, old_inode, old_end, new_generation, new_device, new_inode, new_end, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                storage::stored(expected.generation)?,
                expected.position.device.to_string(),
                expected.position.inode.to_string(),
                expected.position.end_offset.to_string(),
                storage::stored(new.generation)?,
                new.position.device.to_string(),
                new.position.inode.to_string(),
                new.position.end_offset.to_string(),
                observed_at.secs,
            ],
        ).map_err(map)?;
        transaction.execute(
            "UPDATE history_records SET appended_at = NULL, export_device = NULL, export_inode = NULL, export_end = NULL",
            [],
        ).map_err(map)?;
        storage::save_cursor(transaction, new)
    })
}
