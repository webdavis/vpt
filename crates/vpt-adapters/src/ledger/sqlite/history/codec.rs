use serde_json::Value;
use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, ExportFailureId, LedgerNamespace, RunId};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::HistoryFact;

mod event;
pub(super) mod fact;
mod recovery;

pub(super) fn encode(fact: &HistoryFact) -> Result<String, LedgerError> {
    Ok(fact::encode(fact)?.to_string())
}

pub(super) fn decode(text: &str) -> Result<HistoryFact, LedgerError> {
    let value: Value = serde_json::from_str(text).map_err(|_| corrupt())?;
    fact::decode(&value)
}

fn corrupt() -> LedgerError {
    LedgerError::Corrupt("unreadable history fact".into())
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, LedgerError> {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .ok_or_else(corrupt)
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, LedgerError> {
    field(value, key)?.as_str().ok_or_else(corrupt)
}

fn signed(value: &Value, key: &str) -> Result<i64, LedgerError> {
    field(value, key)?.as_i64().ok_or_else(corrupt)
}

fn unsigned(value: &Value, key: &str) -> Result<u64, LedgerError> {
    field(value, key)?.as_u64().ok_or_else(corrupt)
}

fn count(value: &Value, key: &str) -> Result<u32, LedgerError> {
    u32::try_from(unsigned(value, key)?).map_err(|_| corrupt())
}

fn optional<'a>(value: &'a Value, key: &str) -> Result<Option<&'a Value>, LedgerError> {
    let field = field(value, key)?;
    Ok((!field.is_null()).then_some(field))
}

fn parse_id(text: &str) -> Result<(LedgerNamespace, u64), LedgerError> {
    let (hex, number) = text.rsplit_once('-').ok_or_else(corrupt)?;
    if hex.len() != 32
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(corrupt());
    }
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).map_err(|_| corrupt())?;
    }
    let sequence = number.parse::<u64>().map_err(|_| corrupt())?;
    if sequence == 0 || sequence.to_string() != number {
        return Err(corrupt());
    }
    Ok((LedgerNamespace::new(bytes), sequence))
}

fn run_id(value: &Value, key: &str) -> Result<RunId, LedgerError> {
    let (namespace, sequence) = parse_id(string(value, key)?)?;
    Ok(RunId::new(namespace, sequence))
}

fn event_id(value: &Value, key: &str) -> Result<EventId, LedgerError> {
    let (namespace, sequence) = parse_id(string(value, key)?)?;
    Ok(EventId::new(namespace, sequence))
}

fn failure_id(value: &Value, key: &str) -> Result<ExportFailureId, LedgerError> {
    let (namespace, sequence) = parse_id(string(value, key)?)?;
    Ok(ExportFailureId::new(namespace, sequence))
}

fn recording(value: &Value, key: &str) -> Result<Option<RecordingId>, LedgerError> {
    optional(value, key)?
        .map(|value| RecordingId::parse(value.as_str().ok_or_else(corrupt)?).map_err(|_| corrupt()))
        .transpose()
}

#[cfg(test)]
mod tests;
