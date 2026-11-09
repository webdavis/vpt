use super::{Class, ClassificationError, LexicalFlag, Range, stream::Stream};
use crate::{
    alignment::{self, Edit, Outcome},
    normalize,
};

pub(super) fn compare(
    record: &Stream<'_>,
    checker: &Stream<'_>,
    ratio: f64,
) -> Result<Option<Vec<LexicalFlag>>, ClassificationError> {
    let comparison = alignment::compare(&record.text, &checker.text, ratio)
        .map_err(ClassificationError::Alignment)?;
    let Outcome::Aligned { edits, .. } = comparison.outcome else {
        return Ok(None);
    };
    let mut flags = Vec::new();
    let mut deleted = Vec::new();
    let mut inserted = Vec::new();
    for edit in edits.into_iter().chain([Edit::Equal {
        main: record.words.len(),
        checker: checker.words.len(),
    }]) {
        match edit {
            Edit::Delete { main } => deleted.push(main),
            Edit::Insert { checker } => inserted.push(checker),
            Edit::Equal {
                main,
                checker: other,
            } => {
                if !deleted.is_empty() || !inserted.is_empty() {
                    let left = extent(&deleted, main);
                    let right = extent(&inserted, other);
                    let class = disagreement(record, checker, left.clone(), right.clone());
                    flags.push(record.flag(left, class, Some(checker.text[right].join(" "))));
                    deleted.clear();
                    inserted.clear();
                }
                if main < record.words.len()
                    && ordinal_difference(record.text[main], checker.text[other])
                {
                    flags.push(record.flag(
                        main..main + 1,
                        Class::Formatting,
                        Some(checker.text[other].into()),
                    ));
                }
            }
        }
    }
    Ok(Some(flags))
}

fn extent(indices: &[usize], gap: usize) -> Range<usize> {
    indices
        .first()
        .zip(indices.last())
        .map_or(gap..gap, |(first, last)| *first..last + 1)
}

fn disagreement(
    record: &Stream<'_>,
    checker: &Stream<'_>,
    left: Range<usize>,
    right: Range<usize>,
) -> Class {
    let a = &record.text[left.clone()];
    let b = &checker.text[right.clone()];
    let insertions = if a.is_empty() {
        b
    } else if b.is_empty() {
        a
    } else {
        &[]
    };
    if !insertions.is_empty()
        && insertions.iter().all(|word| {
            matches!(
                alphanumeric(word).as_str(),
                "the" | "of" | "a" | "an" | "and" | "to" | "at" | "is"
            )
        })
    {
        Class::Formatting
    } else if digits(a) != digits(b) {
        Class::Numeric
    } else if left.into_iter().any(|i| record.proper(i))
        || right.into_iter().any(|i| checker.proper(i))
    {
        Class::ProperNoun
    } else {
        Class::Other
    }
}

fn alphanumeric(text: &str) -> String {
    normalize::canonical(text)
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn digits(words: &[&str]) -> Vec<String> {
    words
        .iter()
        .flat_map(|word| {
            alphanumeric(word)
                .split(|ch: char| !ch.is_numeric())
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn ordinal_difference(left: &str, right: &str) -> bool {
    let left = alphanumeric(left);
    let right = alphanumeric(right);
    if left == right {
        return false;
    }
    let strip = |text: &str| {
        ["st", "nd", "rd", "th"].iter().any(|suffix| {
            text.strip_suffix(suffix).is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            })
        })
    };
    strip(&left) || strip(&right)
}
