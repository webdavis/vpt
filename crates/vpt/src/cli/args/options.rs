use super::{Format, UsageError};

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

pub(super) fn take_format(words: &mut Vec<String>) -> Result<Option<Format>, UsageError> {
    let mut taken: Vec<(usize, &str, Format)> = Format::FLAGS
        .into_iter()
        .filter_map(|(flag, format)| {
            let at = words.iter().position(|word| word == flag)?;
            Some((at, flag, format))
        })
        .collect();
    taken.sort_unstable_by_key(|(at, ..)| *at);
    if let [(_, first, _), (_, second, _), ..] = taken.as_slice() {
        return Err(UsageError(format!("{second} conflicts with {first}")));
    }
    Ok(taken.first().map(|&(_, flag, format)| {
        take_flag(words, flag);
        format
    }))
}
