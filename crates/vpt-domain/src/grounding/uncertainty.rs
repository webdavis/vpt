use super::{GroundingError, SourceRange, sources::intersects, spend};
use crate::{
    digest::Sha256Digest,
    review::{FlagContent, Resolution, ReviewFlag},
    transcript::Word,
};
use std::collections::BTreeSet;

pub(super) fn inherit(
    words: &[&Word],
    flags: &[ReviewFlag],
    sources: &[SourceRange],
    remaining: &mut usize,
) -> Result<Vec<Sha256Digest>, GroundingError> {
    let mut seen = BTreeSet::new();
    for flag in flags {
        spend(remaining, 1)?;
        let FlagContent::Lexical(lexical) = &flag.content else {
            continue;
        };
        if flag.resolution != Resolution::Open || !lexical.class.surfaced() {
            continue;
        }
        let mut inherited = false;
        for range in &lexical.ranges {
            spend(remaining, 1)?;
            for source in sources {
                spend(remaining, 1)?;
                let start = source.start_secs as f64;
                let end = source.end_secs as f64;
                if range.is_empty() {
                    let at = words
                        .get(range.start)
                        .map_or(words[words.len() - 1].end, |word| word.start);
                    inherited = intersects(at, at, start, end);
                } else {
                    for word in &words[range.clone()] {
                        spend(remaining, 1)?;
                        if intersects(word.start, word.end, start, end) {
                            inherited = true;
                            break;
                        }
                    }
                }
                if inherited {
                    break;
                }
            }
            if inherited {
                break;
            }
        }
        if inherited {
            seen.insert(flag.identifier.0);
        }
    }
    Ok(seen.into_iter().map(Sha256Digest).collect())
}
