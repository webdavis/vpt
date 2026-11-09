use super::*;
use vpt_domain::{history::RetentionCounts, layout::StoreKey};

pub(super) fn encode(counts: RetentionCounts) -> Value {
    Value::Object(
        StoreKey::all()
            .into_iter()
            .map(|store| (store.key_name().to_owned(), json!(counts.get(store))))
            .collect(),
    )
}

pub(super) fn decode(value: Option<&Value>) -> Result<Option<RetentionCounts>, LedgerError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let mut counts = RetentionCounts::default();
    for store in StoreKey::all() {
        counts.set(store, count(value, store.key_name())?);
    }
    Ok(Some(counts))
}
