use super::*;

const DEFAULT_LIMIT: usize = 50;

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub words: Vec<String>,
    pub limit: usize,
}

pub(super) fn parse(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    let limit = match take_value(words, "--limit")? {
        None => DEFAULT_LIMIT,
        Some(value) => value
            .parse()
            .ok()
            .filter(|limit| *limit > 0)
            .ok_or_else(|| UsageError("--limit must be a positive whole number".into()))?,
    };
    if let Some(flag) = words.iter().find(|word| word.starts_with("--")) {
        return Err(UsageError(format!("unexpected argument {flag}")));
    }
    if words.is_empty() {
        return Err(UsageError("<word> is required".into()));
    }
    Ok(Verb::Search(Options {
        words: std::mem::take(words),
        limit,
    }))
}
