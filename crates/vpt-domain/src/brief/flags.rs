use super::{BriefError, TranscriptSource, direct::spend};
use crate::{
    note::NoteStage,
    review::{FlagContent, Resolution, ReviewFlag},
    uncertainty::Diagnostic,
};
use std::{collections::BTreeSet, ops::Range};

fn relevant(flag: &ReviewFlag) -> bool {
    match &flag.content {
        FlagContent::Lexical(flag) => flag.class.surfaced(),
        FlagContent::Diagnostic(_) => true,
        FlagContent::Verification(flag) => flag.artifact == NoteStage::Transcript,
    }
}
pub(super) fn reviewed(source: &TranscriptSource) -> bool {
    source.reviewed
        && !source.flags.iter().any(|flag| {
            flag.resolution == Resolution::Open
                && matches!(flag.content, FlagContent::Diagnostic(Diagnostic::Divergent))
        })
}
pub(super) fn unresolved(source: &TranscriptSource) -> usize {
    source
        .flags
        .iter()
        .filter(|flag| relevant(flag) && flag.resolution == Resolution::Open)
        .map(|flag| flag.identifier.0)
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn uncertain(
    source: &TranscriptSource,
    segment: usize,
    words: Range<usize>,
    total_words: usize,
    remaining: &mut usize,
) -> Result<bool, BriefError> {
    if !source.reviewed {
        return Ok(true);
    }
    let span = &source.transcript.segments[segment];
    for flag in &source.flags {
        spend(remaining, 1)?;
        let corrected = matches!(flag.resolution, Resolution::Corrected { .. });
        if (!corrected && flag.resolution != Resolution::Open) || (!corrected && !relevant(flag)) {
            continue;
        }
        match &flag.content {
            FlagContent::Lexical(flag) => {
                for range in &flag.ranges {
                    spend(remaining, 1)?;
                    let hit = if range.is_empty() {
                        words.start <= range.start
                            && (range.start < words.end
                                || (range.start == total_words && words.end == total_words))
                    } else {
                        words.start < range.end && range.start < words.end
                    };
                    if hit {
                        return Ok(true);
                    }
                }
            }
            FlagContent::Diagnostic(_) => return Ok(true),
            FlagContent::Verification(flag) if flag.artifact == NoteStage::Transcript => {
                if flag.sources.is_empty() {
                    return Ok(true);
                }
                for range in &flag.sources {
                    spend(remaining, 1)?;
                    if (range.start_secs as f64) < span.end && span.start < range.end_secs as f64 {
                        return Ok(true);
                    }
                }
            }
            _ => (),
        }
    }
    Ok(false)
}
