use super::super::{map, storage};
use rusqlite::Connection;
use serde_json::{Value, json};
use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, ExportFailureId};
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportFailureClass, ExportPosition, ExportRestart,
    ObservedLogFile, PartialAppend, SavedExportFailure,
};
use vpt_domain::time::UtcInstant;

pub(super) fn class_tag(class: ExportFailureClass) -> &'static str {
    match class {
        ExportFailureClass::LeafRefused => "leaf_refused",
        ExportFailureClass::Io => "io",
        ExportFailureClass::Encoding => "encoding",
        ExportFailureClass::UnsafeState => "unsafe_state",
    }
}

pub(super) fn encode_partial(partial: PartialAppend) -> String {
    json!({
        "original_end":partial.original_end,
        "retained_end":partial.retained_end,
        "tail_repair_synced":partial.tail_repair_synced,
        "bytes_written":partial.bytes_written,
        "file_synced":partial.file_synced,
        "directory_synced":partial.directory_synced,
        "identity_revalidated":partial.identity_revalidated,
    })
    .to_string()
}

pub(super) fn encode_observed_file(file: ObservedLogFile) -> String {
    json!({
        "device":file.device,
        "inode":file.inode,
        "complete_end_offset":file.complete_end_offset,
        "physical_end_offset":file.physical_end_offset,
    })
    .to_string()
}

fn decode_observed_file(text: &str) -> Result<ObservedLogFile, LedgerError> {
    let value: Value = serde_json::from_str(text).map_err(|_| storage::corrupt())?;
    let number = |key| {
        value
            .get(key)
            .and_then(Value::as_u64)
            .ok_or_else(storage::corrupt)
    };
    Ok(ObservedLogFile {
        device: number("device")?,
        inode: number("inode")?,
        complete_end_offset: number("complete_end_offset")?,
        physical_end_offset: number("physical_end_offset")?,
    })
}

fn decode_partial(text: &str) -> Result<PartialAppend, LedgerError> {
    let value: Value = serde_json::from_str(text).map_err(|_| storage::corrupt())?;
    let number = |key| {
        value
            .get(key)
            .and_then(Value::as_u64)
            .ok_or_else(storage::corrupt)
    };
    let flag = |key| {
        value
            .get(key)
            .and_then(Value::as_bool)
            .ok_or_else(storage::corrupt)
    };
    Ok(PartialAppend {
        original_end: number("original_end")?,
        retained_end: number("retained_end")?,
        tail_repair_synced: flag("tail_repair_synced")?,
        bytes_written: number("bytes_written")?,
        file_synced: flag("file_synced")?,
        directory_synced: flag("directory_synced")?,
        identity_revalidated: flag("identity_revalidated")?,
    })
}

fn position(
    device: Option<String>,
    inode: Option<String>,
    end: Option<String>,
) -> Result<Option<ExportPosition>, LedgerError> {
    match (device, inode, end) {
        (None, None, None) => Ok(None),
        (Some(device), Some(inode), Some(end)) => Ok(Some(ExportPosition {
            device: storage::parse_unsigned(&device)?,
            inode: storage::parse_unsigned(&inode)?,
            end_offset: storage::parse_unsigned(&end)?,
        })),
        _ => Err(storage::corrupt()),
    }
}

pub(super) fn failures(connection: &Connection) -> Result<Vec<SavedExportFailure>, LedgerError> {
    let namespace = storage::namespace(connection)?;
    let mut statement = connection.prepare("SELECT sequence, event_sequence, observed_at, class, last_device, last_inode, last_end, observed_file_json, partial_json FROM history_export_failures ORDER BY sequence").map_err(map)?;
    let mut rows = statement.query([]).map_err(map)?;
    let mut failures = Vec::new();
    while let Some(row) = rows.next().map_err(map)? {
        let class: String = row.get(3).map_err(storage::column)?;
        let class = match class.as_str() {
            "leaf_refused" => ExportFailureClass::LeafRefused,
            "io" => ExportFailureClass::Io,
            "encoding" => ExportFailureClass::Encoding,
            "unsafe_state" => ExportFailureClass::UnsafeState,
            _ => return Err(storage::corrupt()),
        };
        let observed_file: Option<String> = row.get(7).map_err(storage::column)?;
        let partial: Option<String> = row.get(8).map_err(storage::column)?;
        failures.push(SavedExportFailure {
            id: ExportFailureId::new(
                namespace,
                storage::positive(row.get(0).map_err(storage::column)?)?,
            ),
            failure: ExportFailure {
                event: EventId::new(
                    namespace,
                    storage::positive(row.get(1).map_err(storage::column)?)?,
                ),
                observed_at: UtcInstant {
                    secs: row.get(2).map_err(storage::column)?,
                },
                class,
                last_confirmed: position(
                    row.get(4).map_err(storage::column)?,
                    row.get(5).map_err(storage::column)?,
                    row.get(6).map_err(storage::column)?,
                )?,
                observed_file: observed_file
                    .as_deref()
                    .map(decode_observed_file)
                    .transpose()?,
                partial: partial.as_deref().map(decode_partial).transpose()?,
            },
        });
    }
    Ok(failures)
}

pub(super) fn restarts(connection: &Connection) -> Result<Vec<ExportRestart>, LedgerError> {
    let mut statement = connection.prepare("SELECT old_generation, old_device, old_inode, old_end, new_generation, new_device, new_inode, new_end, observed_at FROM history_export_restarts ORDER BY sequence").map_err(map)?;
    let mut rows = statement.query([]).map_err(map)?;
    let mut restarts = Vec::new();
    while let Some(row) = rows.next().map_err(map)? {
        let generation = |index| -> Result<u64, LedgerError> {
            u64::try_from(row.get::<_, i64>(index).map_err(storage::column)?)
                .map_err(|_| storage::corrupt())
        };
        let position = |first| -> Result<ExportPosition, LedgerError> {
            Ok(ExportPosition {
                device: storage::parse_unsigned(
                    &row.get::<_, String>(first).map_err(storage::column)?,
                )?,
                inode: storage::parse_unsigned(
                    &row.get::<_, String>(first + 1).map_err(storage::column)?,
                )?,
                end_offset: storage::parse_unsigned(
                    &row.get::<_, String>(first + 2).map_err(storage::column)?,
                )?,
            })
        };
        restarts.push(ExportRestart {
            old: ExportCursor {
                generation: generation(0)?,
                position: position(1)?,
            },
            new: ExportCursor {
                generation: generation(4)?,
                position: position(5)?,
            },
            observed_at: UtcInstant {
                secs: row.get(8).map_err(storage::column)?,
            },
        });
    }
    Ok(restarts)
}
