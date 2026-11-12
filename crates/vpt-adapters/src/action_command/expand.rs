const LIMIT: usize = 1_048_576;

pub(super) const OPTIONAL: &[&str] = &["{owner_name}", "{due}", "{due_date}", "{due_time}"];

/// An argument that is exactly one absent optional placeholder, or a `-` flag joined to one by `=`,
/// is left out. A bare placeholder also takes the argument before it when that one is a flag: it
/// starts with `-`, is not `--`, holds no placeholder and is not the program.
pub(super) fn expand(
    command: &[String],
    values: &[(&str, &str)],
    optional: &[&str],
) -> Option<Vec<String>> {
    let absent = |token: &str| {
        optional.contains(&token)
            && values
                .iter()
                .any(|(name, value)| *name == token && value.is_empty())
    };
    let placeholder = |argument: &str| values.iter().any(|(name, _)| argument.contains(name));
    let mut keep = vec![true; command.len()];
    for (index, argument) in command.iter().enumerate().skip(1) {
        let joined = argument.split_once('=').is_some_and(|(flag, token)| {
            flag.starts_with('-') && !placeholder(flag) && absent(token)
        });
        if joined {
            keep[index] = false;
        } else if absent(argument) {
            keep[index] = false;
            let before = &command[index - 1];
            if index > 1 && before.starts_with('-') && before != "--" && !placeholder(before) {
                keep[index - 1] = false;
            }
        }
    }
    let mut remaining = LIMIT;
    let argv = command
        .iter()
        .zip(keep)
        .filter(|(_, kept)| *kept)
        .map(|(argument, _)| crate::arguments::replace(argument, values, &mut remaining))
        .collect::<Option<Vec<_>>>()?;
    crate::arguments::valid(&argv, LIMIT).then_some(argv)
}
