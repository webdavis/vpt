use super::super::{codec, map};
use super::{column, corrupt, namespace, parse_unsigned, positive};
use rusqlite::{Connection, Row};
use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, LedgerNamespace, RunId};
use vpt_domain::run::{ExportPosition, HistoryRecord};
use vpt_domain::time::UtcInstant;

fn from_row(row: &Row<'_>, namespace: LedgerNamespace) -> Result<HistoryRecord, LedgerError> {
    let event_seq: i64 = row.get(0).map_err(column)?;
    let run_seq: i64 = row.get(1).map_err(column)?;
    let occurred_at: i64 = row.get(2).map_err(column)?;
    let fact: String = row.get(3).map_err(column)?;
    let appended_at: Option<i64> = row.get(4).map_err(column)?;
    let device: Option<String> = row.get(5).map_err(column)?;
    let inode: Option<String> = row.get(6).map_err(column)?;
    let end: Option<String> = row.get(7).map_err(column)?;
    let export_position = match (device, inode, end) {
        (None, None, None) if appended_at.is_none() => None,
        (Some(device), Some(inode), Some(end)) if appended_at.is_some() => Some(ExportPosition {
            device: parse_unsigned(&device)?,
            inode: parse_unsigned(&inode)?,
            end_offset: parse_unsigned(&end)?,
        }),
        _ => return Err(corrupt()),
    };
    Ok(HistoryRecord {
        id: EventId::new(namespace, positive(event_seq)?),
        run: RunId::new(namespace, positive(run_seq)?),
        occurred_at: UtcInstant { secs: occurred_at },
        fact: codec::decode(&fact)?,
        appended_at: appended_at.map(|secs| UtcInstant { secs }),
        export_position,
    })
}

pub(in crate::ledger::sqlite::history) fn records(
    connection: &Connection,
    pending_only: bool,
) -> Result<Vec<HistoryRecord>, LedgerError> {
    let namespace = namespace(connection)?;
    let sql = if pending_only {
        "SELECT sequence, run_sequence, occurred_at, fact, appended_at, export_device, export_inode, export_end FROM history_records WHERE appended_at IS NULL ORDER BY sequence"
    } else {
        "SELECT sequence, run_sequence, occurred_at, fact, appended_at, export_device, export_inode, export_end FROM history_records ORDER BY sequence"
    };
    let mut statement = connection.prepare(sql).map_err(map)?;
    let mut rows = statement.query([]).map_err(map)?;
    let mut records = Vec::new();
    while let Some(row) = rows.next().map_err(map)? {
        records.push(from_row(row, namespace)?);
    }
    Ok(records)
}
