mod boundaries;
mod brief_region;
mod direct;
mod quotes;
mod recent;
mod rendering;
use super::*;
use crate::{
    digest::Sha256Digest,
    identity::RecordingId,
    occasion::{OccasionDetails, Participant, time::OccasionTime},
    relations::{Kind, Provenance, Relation, State, Target},
    tags,
    time::{UtcInstant, UtcOffset},
    transcript::*,
};

fn candidate(seed: u8, captured: i64) -> Candidate {
    Candidate {
        recording: RecordingId::derive(
            UtcInstant { secs: captured },
            UtcOffset { secs: 0 },
            &Sha256Digest([seed; 32]),
        )
        .unwrap(),
        captured_at: UtcInstant { secs: captured },
        source: Some(TranscriptSource {
            note: format!("/notes/{seed}.md").into(),
            digest: Sha256Digest([seed; 32]),
            reviewed: true,
            flags: vec![],
            transcript: Transcript {
                engine: EngineMetadata {
                    name: "test".into(),
                    tool: "test".into(),
                    version: "1".into(),
                    model: "test".into(),
                    family: "test".into(),
                },
                language: "en".into(),
                language_source: LanguageSource::Reported,
                local: true,
                segments: vec![Segment {
                    speaker: None,
                    start: 0.0,
                    end: 1.0,
                    text: "Private quote".into(),
                    words: vec![],
                }],
            },
        }),
        tags: vec![],
        relations: vec![],
    }
}
fn occasion() -> OccasionDetails {
    OccasionDetails {
        at: OccasionTime::parse("2026-10-01T09:00:00Z").unwrap(),
        title: "Private project planning".into(),
        duration_secs: None,
        participants: vec![Participant {
            name: "Private Person".into(),
            email: None,
        }],
        tags: vec!["planning".into()],
    }
}
fn term(text: &str, path: &str) -> Term {
    Term {
        text: text.into(),
        target: TermTarget::Unique(path.into()),
    }
}
fn mention(path: &str) -> Relation {
    Relation {
        kind: Kind::Mentions,
        target: Target::Note(path.into()),
        state: State::Confirmed,
        provenance: Provenance::Manual,
        ranges: vec![],
    }
}
fn tag(name: &str, state: tags::State) -> tags::Tag {
    tags::Tag {
        name: name.into(),
        state,
        provenance: tags::Provenance::Manual,
        updated_at: None,
    }
}
fn request<'a>(
    occasion: &'a OccasionDetails,
    terms: &'a [Term],
    candidates: &'a [Candidate],
) -> Request<'a> {
    Request {
        occasion,
        terms,
        candidates,
        now: UtcInstant { secs: 1_000_000 },
        options: Options::default(),
    }
}
