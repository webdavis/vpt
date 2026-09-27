use crate::cli::output::Outcome;
use serde_json::{Map, Value, json};
use vpt_application::StorePaths;
use vpt_application::inventory;
use vpt_application::ports::Stores;
use vpt_domain::layout::StoreKey;
use vpt_domain::time::{FileTime, UtcInstant};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

fn stamp(time: Option<FileTime>) -> Value {
    time.map_or(Value::Null, |time| {
        Value::String(UtcInstant { secs: time.secs }.rfc3339())
    })
}

pub(crate) fn run(paths: &StorePaths, source: &dyn Stores) -> Outcome {
    let mut stores = Map::new();
    let mut human = String::new();
    for key in StoreKey::all() {
        let root = paths.get(key);
        let Some(root_text) = root.to_str() else {
            return Outcome::Failure(ErrorDocument::new(
                ErrorKind::Store,
                "store path is not UTF-8",
            ));
        };
        let entries = match source.entries(root) {
            Ok(entries) => entries,
            Err(error) => return Outcome::Failure(crate::compose::errors::store_error(error)),
        };
        let Some(report) = inventory(&entries) else {
            return Outcome::Failure(ErrorDocument::new(
                ErrorKind::Store,
                "store inventory exceeds supported totals",
            ));
        };
        human.push_str(&format!(
            "{:<15} {:>6} files {:>12} bytes  {}\n",
            key.key_name(),
            report.files,
            report.bytes,
            root_text.escape_debug()
        ));
        stores.insert(
            key.key_name().to_owned(),
            json!({
                "files": report.files,
                "bytes": report.bytes,
                "oldest": stamp(report.oldest),
                "newest": stamp(report.newest),
                "path": root_text,
            }),
        );
    }
    Outcome::Success {
        human,
        document: document("storage", json!({"stores": stores})),
    }
}
