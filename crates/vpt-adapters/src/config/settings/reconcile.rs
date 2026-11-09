use super::{ConfigError, Table, expand, leaf, text, wrong};
use std::path::Path;
use vpt_application::ReconcileSettings;
use vpt_domain::uncertainty::Options;

pub(super) fn from_table(table: &Table, home_dir: &Path) -> Result<ReconcileSettings, ConfigError> {
    Ok(ReconcileSettings {
        options: Options {
            confidence_floor: ratio(table, "reconcile.confidence_floor")?,
            max_divergence_ratio: ratio(table, "reconcile.max_divergence_ratio")?,
        },
        known_terms_path: expand(text(table, "reconcile.known_terms_path")?, home_dir, None),
    })
}

fn ratio(table: &Table, key: &str) -> Result<f64, ConfigError> {
    let value = leaf(table, key)?;
    value
        .as_float()
        .or_else(|| value.as_integer().map(|value| value as f64))
        .ok_or_else(|| wrong(key, "ratio"))
}
