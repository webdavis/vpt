use super::corrections::EvidenceWord;
use super::{Grounder, GroundingError, SourceRange, sources::intersects};
use crate::{review::ReviewFlag, transcript::Transcript};

impl<'a> Grounder<'a> {
    pub fn for_transcript(
        transcript: &'a Transcript,
        flags: &'a [ReviewFlag],
        duration: f64,
        window_secs: u32,
    ) -> Result<Self, GroundingError> {
        let segments = &transcript.segments;
        let mut grounder = Self::new(
            segments.iter().flat_map(|segment| &segment.words),
            flags,
            duration,
            window_secs,
        )?;
        grounder.labels = segments
            .iter()
            .filter_map(|segment| {
                Some(EvidenceWord {
                    text: segment.speaker.as_deref()?,
                    start: segment.start,
                    end: segment.end,
                })
            })
            .collect();
        Ok(grounder)
    }
}

pub(super) fn without_cited(
    text: &str,
    labels: &[EvidenceWord<'_>],
    sources: &[SourceRange],
) -> String {
    let mut text = text.to_owned();
    for label in labels {
        let cited = sources.iter().any(|source| {
            intersects(
                label.start,
                label.end,
                source.start_secs as f64,
                source.end_secs as f64,
            )
        });
        if cited {
            text = remove_phrase(&text, label.text);
        }
    }
    text
}

fn remove_phrase(text: &str, phrase: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(phrase) {
        let end = at + phrase.len();
        let bounded = !rest[..at].ends_with(char::is_alphanumeric)
            && !rest[end..].starts_with(char::is_alphanumeric);
        result.push_str(&rest[..at]);
        result.push_str(if bounded { " " } else { &rest[at..end] });
        rest = &rest[end..];
    }
    result.push_str(rest);
    result
}
