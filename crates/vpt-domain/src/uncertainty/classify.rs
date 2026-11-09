use super::{
    Class, Classification, ClassificationError, Diagnostic, LexicalFlag, Options, Transcript,
    comparison, known, risk,
    stream::{Stream, ends_sentence},
};
use std::collections::BTreeMap;

pub fn classify(
    record: &Transcript,
    checker: Option<&Transcript>,
    options: Options,
    known_terms: &[&str],
) -> Result<Classification, ClassificationError> {
    if !options.confidence_floor.is_finite()
        || !(0.0..=1.0).contains(&options.confidence_floor)
        || !options.max_divergence_ratio.is_finite()
        || options.max_divergence_ratio <= 0.0
        || options.max_divergence_ratio > 1.0
    {
        return Err(ClassificationError::InvalidOptions);
    }
    let stream = Stream::new(record)?;
    let mut result = Classification::default();
    if let Some(checker) = checker {
        let checker = Stream::new(checker)?;
        if let Some(flags) = comparison::compare(&stream, &checker, options.max_divergence_ratio)? {
            result.lexical = flags;
        } else {
            result.diagnostics.push(Diagnostic::Divergent);
        }
    }
    let known = known::coverage(&stream.text, known_terms)?;
    let mut occupied = vec![false; stream.words.len()];
    for flag in &result.lexical {
        for range in &flag.ranges {
            occupied[range.clone()].fill(true)
        }
    }
    for (index, word) in stream.words.iter().enumerate() {
        if !occupied[index]
            && word
                .confidence
                .is_some_and(|value| value < options.confidence_floor)
        {
            result
                .lexical
                .push(stream.flag(index..index + 1, Class::LowConfidence, None));
            occupied[index] = true;
        }
    }
    let english = record
        .language
        .split('-')
        .next()
        .is_some_and(|base| base.eq_ignore_ascii_case("en"));
    let mut start = 0;
    while start < stream.words.len() {
        let Some(kind) =
            risk::kind(&stream, start, english).filter(|_| !occupied[start] && !known[start])
        else {
            start += 1;
            continue;
        };
        let mut end = start + 1;
        while end < stream.words.len()
            && !occupied[end]
            && !known[end]
            && !ends_sentence(stream.text[end - 1])
            && risk::kind(&stream, end, english) == Some(kind)
        {
            end += 1
        }
        result
            .lexical
            .push(stream.flag(start..end, Class::AgreedUnverified, None));
        start = end;
    }
    result.lexical = aggregate(result.lexical);
    result
        .lexical
        .sort_by_key(|flag| (flag.class, flag.ranges[0].start));
    Ok(result)
}

fn aggregate(flags: Vec<LexicalFlag>) -> Vec<LexicalFlag> {
    let mut result: Vec<LexicalFlag> = Vec::new();
    let mut by_surface = BTreeMap::new();
    for flag in flags {
        if flag.class == Class::AgreedUnverified {
            if let Some(&index) = by_surface.get(&flag.record_text) {
                let existing: &mut LexicalFlag = &mut result[index];
                existing.ranges.extend(flag.ranges);
                existing.confidence = existing
                    .confidence
                    .into_iter()
                    .chain(flag.confidence)
                    .reduce(f64::min);
                continue;
            }
            by_surface.insert(flag.record_text.clone(), result.len());
        }
        result.push(flag);
    }
    result
}
