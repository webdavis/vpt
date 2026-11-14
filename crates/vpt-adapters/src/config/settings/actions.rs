use super::*;
use vpt_application::SecretEnv;

pub(super) fn withdraw_command(table: &Table) -> Result<Vec<String>, ConfigError> {
    let command = strings(table, "actions.withdraw_command")?;
    if !command.is_empty() && strings(table, "actions.command")?.is_empty() {
        return Err(ConfigError::OutOfRange {
            key: "actions.withdraw_command".into(),
            rule: "requires [actions] command".into(),
        });
    }
    if let Some(unfilled) = command.iter().find_map(|argument| unfillable(argument)) {
        return Err(ConfigError::OutOfRange {
            key: "actions.withdraw_command".into(),
            rule: format!("{unfilled} cannot be filled; only {{id}} and {{recording}} can"),
        });
    }
    Ok(command)
}

fn unfillable(argument: &str) -> Option<&str> {
    argument.match_indices('{').find_map(|(start, _)| {
        let rest = &argument[start + 1..];
        let end = rest.find('}')?;
        let name = &rest[..end];
        let placeholder =
            !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
        (placeholder && !["id", "recording"].contains(&name))
            .then(|| &argument[start..start + end + 2])
    })
}

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
