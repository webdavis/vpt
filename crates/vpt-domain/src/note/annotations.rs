use super::{RenderError, projection::Projection, writer::Writer};
use crate::{
    readability,
    review::{FlagContent, Resolution, ReviewFlag},
    uncertainty::LexicalFlag,
};
use std::ops::Range;
mod plain;
mod render;
pub(super) use plain::plain;
pub(super) use render::render;

pub(super) struct Span<'a> {
    bytes: Range<usize>,
    flag: crate::digest::Sha256Digest,
    occurrence: usize,
    lexical: &'a LexicalFlag,
    resolution: &'a Resolution,
    first: bool,
}

pub(super) fn spans<'a>(
    projection: &Projection<'_>,
    flags: &'a [ReviewFlag],
) -> Result<Vec<Vec<Span<'a>>>, RenderError> {
    let mut segments: Vec<Vec<Span<'a>>> =
        (0..projection.segments.len()).map(|_| Vec::new()).collect();
    for flag in flags {
        let FlagContent::Lexical(lexical) = &flag.content else {
            continue;
        };
        for (occurrence, range) in lexical.ranges.iter().enumerate() {
            if range.is_empty() {
                let (position, offset) = if let Some(next) = projection.words.get(range.start) {
                    (next, next.bytes.start)
                } else {
                    let last = projection.words.last().ok_or(RenderError::InvalidFlags)?;
                    (last, last.bytes.end)
                };
                segments[position.segment].push(Span {
                    flag: flag.identifier,
                    occurrence,
                    bytes: offset..offset,
                    lexical,
                    resolution: &flag.resolution,
                    first: true,
                });
                continue;
            }
            let mut start = range.start;
            while start < range.end {
                let position = &projection.words[start];
                let end = start
                    + projection.words[start..range.end]
                        .partition_point(|word| word.segment == position.segment);
                segments[position.segment].push(Span {
                    flag: flag.identifier,
                    occurrence,
                    bytes: position.bytes.start..projection.words[end - 1].bytes.end,
                    lexical,
                    resolution: &flag.resolution,
                    first: start == range.start,
                });
                start = end;
            }
        }
    }
    for spans in &mut segments {
        spans.sort_by_key(|span| (span.bytes.start, span.bytes.end));
    }
    Ok(segments)
}
