use super::ConfigError;
use super::schema::{Kind, spec_for};
use toml::{Table, Value};
use vpt_domain::duration::parse_duration;

pub(super) fn validate(table: &Table) -> Result<(), ConfigError> {
    walk(table, "")
}

fn walk(table: &Table, prefix: &str) -> Result<(), ConfigError> {
    for (name, value) in table {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            Value::Table(inner) => walk(inner, &path)?,
            leaf => {
                let kind = spec_for(&path).ok_or_else(|| ConfigError::UnknownKey(path.clone()))?;
                check(&path, kind, leaf)?;
            }
        }
    }
    Ok(())
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|n| n as f64))
}

fn check(key: &str, kind: Kind, value: &Value) -> Result<(), ConfigError> {
    let wrong = || ConfigError::WrongType {
        key: key.into(),
        expected: kind.expected().into(),
    };
    let range = |rule: &str| ConfigError::OutOfRange {
        key: key.into(),
        rule: rule.into(),
    };
    match kind {
        Kind::Bool => value.as_bool().map(|_| ()).ok_or_else(wrong),
        Kind::PositiveInt => match value.as_integer() {
            Some(n) if (1..=i64::from(u32::MAX)).contains(&n) => Ok(()),
            Some(_) => Err(range("a positive 32-bit integer")),
            None => Err(wrong()),
        },
        Kind::NonNegativeInt => match value.as_integer() {
            Some(n) if (0..=i64::from(u32::MAX)).contains(&n) => Ok(()),
            Some(_) => Err(range("a non-negative 32-bit integer")),
            None => Err(wrong()),
        },
        Kind::PositiveInt64 => match value.as_integer() {
            Some(n) if n >= 1 => Ok(()),
            Some(_) => Err(range("a positive 64-bit integer")),
            None => Err(wrong()),
        },
        Kind::Ratio => match number(value) {
            Some(f) if f.is_finite() && (0.0..=1.0).contains(&f) => Ok(()),
            Some(_) => Err(range("a ratio in [0, 1]")),
            None => Err(wrong()),
        },
        Kind::RatioAboveZero => match number(value) {
            Some(f) if f.is_finite() && f > 0.0 && f <= 1.0 => Ok(()),
            Some(_) => Err(range("a ratio in (0, 1]")),
            None => Err(wrong()),
        },
        Kind::Duration => match value {
            Value::Integer(0) => Ok(()),
            Value::String(text) => parse_duration(text)
                .map(|_| ())
                .map_err(|_| range("0 or <n>s|m|h|d")),
            _ => Err(range("0 or <n>s|m|h|d")),
        },
        Kind::Enum(options) => match value.as_str() {
            Some(text) if options.contains(&text) => Ok(()),
            Some(_) => Err(range(&format!("one of {}", options.join(", ")))),
            None => Err(wrong()),
        },
        Kind::Text | Kind::Secret => value.as_str().map(|_| ()).ok_or_else(wrong),
        Kind::TextList | Kind::Argv => match value.as_array() {
            Some(items) if items.iter().all(Value::is_str) => Ok(()),
            _ => Err(wrong()),
        },
        Kind::ExitCodes => match value.as_array() {
            Some(items) if items.iter().all(|item| item.as_integer().is_some()) => {
                if items
                    .iter()
                    .filter_map(Value::as_integer)
                    .all(|code| (1..=255).contains(&code))
                {
                    Ok(())
                } else {
                    Err(range("exit codes from 1 to 255"))
                }
            }
            _ => Err(wrong()),
        },
    }
}

#[cfg(test)]
mod tests;
