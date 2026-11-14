use super::*;
use vpt_application::SecretEnv;

pub(super) fn final_exit_codes(table: &Table) -> Result<Vec<i32>, ConfigError> {
    Ok(leaf(table, "actions.final_exit_codes")?
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_integer)
        .filter_map(|code| i32::try_from(code).ok())
        .collect())
}

pub(super) fn owner_words(table: &Table) -> Vec<(String, String)> {
    string_entries(table, "actions.owner_words")
}

pub(super) fn env(table: &Table) -> SecretEnv {
    SecretEnv(string_entries(table, "actions.env"))
}

fn string_entries(table: &Table, key: &str) -> Vec<(String, String)> {
    get(table, key)
        .and_then(Value::as_table)
        .into_iter()
        .flatten()
        .filter_map(|(name, value)| Some((name.clone(), value.as_str()?.to_owned())))
        .collect()
}
