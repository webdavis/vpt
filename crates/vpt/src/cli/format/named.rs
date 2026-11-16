use super::Format;
use clap::ValueEnum;
use std::ffi::OsString;

/// The one format the words name before `--`, so a usage error can be printed in it before the
/// words are parsed; none when they name no format or more than one.
pub(crate) fn named(args: &[OsString]) -> Option<Format> {
    let words: Vec<&str> = args
        .iter()
        .skip(1)
        .map(|word| word.to_str().unwrap_or_default())
        .take_while(|word| *word != "--")
        .collect();
    let mut formats = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let value = match *word {
            "--output" | "-o" => words.get(index + 1).copied(),
            _ => word
                .strip_prefix("--output=")
                .or_else(|| word.strip_prefix("-o").filter(|value| !value.is_empty())),
        };
        formats.extend(value.and_then(|value| Format::from_str(value, false).ok()));
    }
    match formats.as_slice() {
        [format] => Some(*format),
        _ => None,
    }
}
