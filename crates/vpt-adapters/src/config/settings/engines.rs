mod definition;
use crate::config::{ConfigError, get};
use std::collections::BTreeMap;
use std::time::Duration;
use toml::Table;
use vpt_application::engines::{CheckerRuns, EngineConfig, EnginePair, EngineSettings};

pub(super) fn from_table(table: &Table) -> Result<EngineSettings, ConfigError> {
    let definitions = get(table, "engines")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| required("engines"))?
        .iter()
        .filter(|(name, value)| name.as_str() != "by_language" && value.is_table())
        .map(|(name, _)| definition::read(table, name).map(|engine| (name.clone(), engine)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let pair = EnginePair {
        main: nonempty(table, "engines.main")?.into(),
        checker: optional(super::text(table, "engines.checker")?),
    };
    validate_pair(&pair, "engines", &definitions)?;
    let mut by_language = BTreeMap::new();
    if let Some(overrides) = get(table, "engines.by_language").and_then(toml::Value::as_table) {
        for language in overrides.keys() {
            let prefix = format!("engines.by_language.{language}");
            let main = format!("{prefix}.main");
            let checker = format!("{prefix}.checker");
            let selected = EnginePair {
                main: get(table, &main)
                    .map(|_| nonempty(table, &main).map(str::to_owned))
                    .transpose()?
                    .unwrap_or_else(|| pair.main.clone()),
                checker: if get(table, &checker).is_some() {
                    optional(super::text(table, &checker)?)
                } else {
                    pair.checker.clone()
                },
            };
            validate_pair(&selected, &prefix, &definitions)?;
            by_language.insert(language.clone(), selected);
        }
    }
    Ok(EngineSettings {
        language: nonempty(table, "recording.language")?.into(),
        pair,
        definitions,
        by_language,
        checker_runs: match super::text(table, "engines.checker_runs")? {
            "always" => CheckerRuns::Always,
            "on_failure" => CheckerRuns::OnFailure,
            _ => return Err(required("engines.checker_runs")),
        },
        allow_same_family: super::boolean(table, "engines.allow_same_family")?,
        timeout: Duration::from_secs(u64::from(super::small(table, "engines.timeout_secs")?)),
    })
}

fn validate_pair(
    pair: &EnginePair,
    prefix: &str,
    definitions: &BTreeMap<String, EngineConfig>,
) -> Result<(), ConfigError> {
    for (slot, name) in [
        ("main", Some(pair.main.as_str())),
        ("checker", pair.checker.as_deref()),
    ] {
        if name.is_some_and(|name| !definitions.contains_key(name)) {
            return Err(ConfigError::OutOfRange {
                key: format!("{prefix}.{slot}"),
                rule: "must name a configured engine".into(),
            });
        }
    }
    Ok(())
}

fn optional(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn required(key: &str) -> ConfigError {
    ConfigError::OutOfRange {
        key: key.into(),
        rule: "a required engine setting is missing or invalid".into(),
    }
}

fn nonempty<'a>(table: &'a Table, key: &str) -> Result<&'a str, ConfigError> {
    let text = get(table, key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| required(key))?;
    if text.trim().is_empty() || text.contains('\0') {
        return Err(required(key));
    }
    Ok(text)
}

#[cfg(test)]
mod tests;
