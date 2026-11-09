mod queue;
pub use queue::ReviewQueue;
mod batch;
use super::Review;
pub use batch::{AlertError, events};
use std::path::PathBuf;
use vpt_domain::{history::*, identity::RecordingId, time::UtcInstant, uncertainty::Class};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewNotice {
    pub recording: RecordingId,
    pub counts: EventCounts,
    pub path: PathBuf,
}

impl ReviewNotice {
    pub fn from_review(review: &Review, path: PathBuf) -> Option<Self> {
        if review.flags.is_empty() && review.diagnostics.is_empty() && review.findings.is_empty() {
            return None;
        }
        Some(Self {
            recording: review.recording.clone(),
            counts: counts(review),
            path,
        })
    }

    pub fn event(&self, at: UtcInstant) -> Event {
        Event {
            kind: EventKind::ReviewNeeded,
            state: EventState::NeedsAttention,
            occasion: None,
            recording: Some(self.recording.clone()),
            detail: EventDetail::Review {
                spans: self
                    .counts
                    .numeric
                    .saturating_add(self.counts.proper_noun)
                    .saturating_add(self.counts.low_confidence)
                    .saturating_add(self.counts.other)
                    .saturating_add(self.counts.unsupported)
                    .saturating_add(self.counts.unsourced)
                    .saturating_add(self.counts.bad_reference)
                    .saturating_add(self.counts.built_on_flagged_text),
                agreed_unverified: self.counts.agreed_unverified,
            },
            counts: self.counts,
            paths: vec![EventPath {
                key: EventPathKey::Note,
                path: self.path.clone(),
            }],
            occurred_at: at,
        }
    }
}

#[cfg(test)]
mod tests;

pub fn counts(review: &Review) -> EventCounts {
    let mut counts = EventCounts {
        diagnostics: review.diagnostics.len() as u32,
        ..Default::default()
    };
    for flag in &review.flags {
        let count = flag.occurrences.len() as u32;
        let target = match flag.flag.class {
            Class::Numeric => &mut counts.numeric,
            Class::ProperNoun => &mut counts.proper_noun,
            Class::LowConfidence => &mut counts.low_confidence,
            Class::AgreedUnverified => &mut counts.agreed_unverified,
            Class::Other => &mut counts.other,
            Class::Formatting => continue,
        };
        *target = target.saturating_add(count);
    }
    for (_, flag) in &review.findings {
        use vpt_domain::grounding::Finding;
        let target = match flag.finding {
            Finding::Unsourced => &mut counts.unsourced,
            Finding::BadReference => &mut counts.bad_reference,
            Finding::Unsupported(_) => &mut counts.unsupported,
            Finding::BuiltOnFlaggedText => &mut counts.built_on_flagged_text,
        };
        *target = target.saturating_add(1);
    }
    counts
}
