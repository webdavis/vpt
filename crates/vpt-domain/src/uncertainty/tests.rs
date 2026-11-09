#![allow(
    clippy::single_range_in_vec_init,
    reason = "Flags store ranges, including a single occurrence"
)]
use super::*;
use crate::transcript::{EngineMetadata, LanguageSource, Segment, Word};

mod disagreement;
mod limits;
mod risks;

fn transcript(text: &str) -> Transcript {
    Transcript {
        engine: EngineMetadata {
            name: "fixture".into(),
            tool: "fixture".into(),
            version: "1".into(),
            model: "fixture".into(),
            family: "fixture".into(),
        },
        language: "en-US".into(),
        language_source: LanguageSource::Reported,
        local: true,
        segments: vec![Segment {
            start: 0.0,
            end: text.split_whitespace().count() as f64,
            text: text.into(),
            words: text
                .split_whitespace()
                .enumerate()
                .map(|(index, text)| Word {
                    text: text.into(),
                    start: index as f64,
                    end: index as f64 + 1.0,
                    confidence: None,
                })
                .collect(),
        }],
    }
}

fn compared(main: &str, checker: &str) -> Classification {
    classify(
        &transcript(main),
        Some(&transcript(checker)),
        Options {
            max_divergence_ratio: 1.0,
            ..Options::default()
        },
        &[],
    )
    .unwrap()
}

fn classes(result: &Classification) -> Vec<Class> {
    result.lexical.iter().map(|flag| flag.class).collect()
}
