use crate::transcript::{Segment, Transcript};
use std::{borrow::Cow, ops::Range};

pub(super) struct Projection<'a> {
    pub segments: Vec<Cow<'a, str>>,
    pub words: Vec<Position>,
}

pub(super) struct Position {
    pub segment: usize,
    pub bytes: Range<usize>,
}

pub(super) fn project(transcript: &Transcript) -> Projection<'_> {
    let mut result = Projection {
        segments: Vec::new(),
        words: Vec::new(),
    };
    for (index, segment) in transcript.segments.iter().enumerate() {
        let (text, ranges) = segment_text(segment);
        result.segments.push(text);
        result
            .words
            .extend(ranges.into_iter().map(|bytes| Position {
                segment: index,
                bytes,
            }));
    }
    result
}

fn segment_text(segment: &Segment) -> (Cow<'_, str>, Vec<Range<usize>>) {
    if let Some(ranges) = locate(segment) {
        return (Cow::Borrowed(&segment.text), ranges);
    }
    // Word ranges own flags. If an engine's segment text disagrees with its words,
    // use the accepted words rather than silently dropping their annotations.
    let mut text = String::new();
    let mut ranges = Vec::new();
    for word in &segment.words {
        if !ranges.is_empty() {
            text.push(' ');
        }
        let start = text.len();
        text.push_str(&word.text);
        ranges.push(start..text.len());
    }
    (Cow::Owned(text), ranges)
}

fn locate(segment: &Segment) -> Option<Vec<Range<usize>>> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    for word in &segment.words {
        let start = cursor + segment.text[cursor..].find(&word.text)?;
        if segment.text[cursor..start]
            .chars()
            .any(char::is_alphanumeric)
        {
            return None;
        }
        cursor = start + word.text.len();
        ranges.push(start..cursor);
    }
    if !ranges.is_empty() && segment.text[cursor..].chars().any(char::is_alphanumeric) {
        return None;
    }
    Some(ranges)
}
