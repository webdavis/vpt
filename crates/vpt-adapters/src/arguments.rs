pub(crate) fn valid(command: &[String], limit: usize) -> bool {
    command.len() <= 256
        && command
            .first()
            .is_some_and(|program| !program.trim().is_empty())
        && command
            .iter()
            .try_fold(limit, |remaining, argument| {
                if argument.contains('\0') {
                    None
                } else {
                    remaining.checked_sub(argument.len())
                }
            })
            .is_some()
}

pub(crate) fn replace(
    mut source: &str,
    values: &[(&str, &str)],
    remaining: &mut usize,
) -> Option<String> {
    let mut result = String::new();
    while let Some(start) = source.find('{') {
        append(&mut result, &source[..start], remaining)?;
        source = &source[start..];
        if let Some((token, value)) = values.iter().find(|(token, _)| source.starts_with(token)) {
            append(&mut result, value, remaining)?;
            source = &source[token.len()..];
        } else {
            append(&mut result, "{", remaining)?;
            source = &source[1..];
        }
    }
    append(&mut result, source, remaining)?;
    Some(result)
}

#[cfg(test)]
mod tests;

fn append(result: &mut String, value: &str, remaining: &mut usize) -> Option<()> {
    *remaining = remaining.checked_sub(value.len())?;
    result.push_str(value);
    Some(())
}
