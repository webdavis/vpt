use super::UsageError;

pub(super) fn take_flag(words: &mut Vec<String>, flag: &str) -> bool {
    match words.iter().position(|word| word == flag) {
        Some(index) => {
            words.remove(index);
            true
        }
        None => false,
    }
}

pub(super) fn take_value(
    words: &mut Vec<String>,
    flag: &str,
) -> Result<Option<String>, UsageError> {
    let Some(index) = words.iter().position(|word| word == flag) else {
        return Ok(None);
    };
    if index + 1 >= words.len() || words[index + 1].starts_with("--") {
        return Err(UsageError(format!("{flag} needs a value")));
    }
    words.remove(index);
    Ok(Some(words.remove(index)))
}

pub(super) fn take_positional(words: &mut Vec<String>, name: &str) -> Result<String, UsageError> {
    if words.is_empty() || words[0].starts_with("--") {
        return Err(UsageError(format!("{name} is required")));
    }
    Ok(words.remove(0))
}
