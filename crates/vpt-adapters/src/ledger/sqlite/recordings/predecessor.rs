use super::{
    Connection, LedgerError, OptionalExtension, RECORDING_COLUMNS, RecordingRecord, map, params,
    record_from_row,
};

pub(in crate::ledger::sqlite) fn read(
    connection: &Connection,
    current: &RecordingRecord,
) -> Result<Option<RecordingRecord>, LedgerError> {
    connection.query_row(
        &format!("SELECT {RECORDING_COLUMNS} FROM recordings WHERE (captured_at, id) < (?1, ?2) ORDER BY captured_at DESC, id DESC LIMIT 1"),
        params![current.captured_at.secs, current.id.as_str()], |row| Ok(record_from_row(row)),
    ).optional().map_err(map)?.transpose()
}
