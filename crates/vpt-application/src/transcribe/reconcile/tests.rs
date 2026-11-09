use super::*;
use crate::{ports::EngineOutput, transcribe::EngineSlot};
use vpt_domain::{
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
    uncertainty::{Class, Diagnostic},
};

fn output(text: &str, language: &str) -> EngineOutput {
    EngineOutput {
        transcript: Transcript {
            engine: EngineMetadata {
                name: "fixture".into(),
                family: "fixture".into(),
                model: "fixture".into(),
                version: "1".into(),
                tool: "fixture".into(),
            },
            language: language.into(),
            language_source: LanguageSource::Reported,
            local: true,
            segments: vec![Segment {
                start: 0.0,
                end: 100.0,
                text: text.into(),
                words: text
                    .split_whitespace()
                    .enumerate()
                    .map(|(index, text)| Word {
                        text: text.into(),
                        start: index as f64,
                        end: index as f64 + 1.0,
                        confidence: Some(0.1),
                    })
                    .collect(),
            }],
        },
        raw_document: vec![],
        diagnostics: vec![],
    }
}

fn result() -> Transcription {
    Transcription {
        language: "en-US".into(),
        record_slot: EngineSlot::Main,
        record: output("we met Alice", "en-US"),
        checker: Some(output("other words entirely", "en-US")),
        lexical_classification: true,
        checker_lexical_classification: true,
        failures: vec![],
    }
}

#[test]
fn builtins_classify_and_report_divergence() {
    let flags = result().classify(Options::default(), &[]).unwrap();
    assert_eq!(flags.diagnostics, [Diagnostic::Divergent]);
    assert_eq!(flags.lexical.len(), 3);
    assert!(
        flags
            .lexical
            .iter()
            .all(|flag| flag.class == Class::LowConfidence)
    );
}

#[test]
fn command_record_keeps_diagnostics_and_has_no_lexical_flags() {
    let mut result = result();
    result.lexical_classification = false;
    result.checker = None;
    result.record.transcript.language = "fr".into();
    let flags = result.classify(Options::default(), &[]).unwrap();
    assert!(flags.lexical.is_empty());
    assert_eq!(
        flags.diagnostics,
        [Diagnostic::SingleEngine, Diagnostic::LanguageMismatch]
    );
}

#[test]
fn command_checker_is_exempt_but_record_confidence_is_still_classified() {
    let mut result = result();
    result.checker_lexical_classification = false;
    let flags = result.classify(Options::default(), &[]).unwrap();
    assert!(flags.diagnostics.is_empty());
    assert_eq!(flags.lexical.len(), 3);
}

#[test]
fn checker_language_mismatch_is_independent_of_lexical_eligibility() {
    let mut result = result();
    result.checker_lexical_classification = false;
    result.checker.as_mut().unwrap().transcript.language = "fr".into();
    assert_eq!(
        result
            .classify(Options::default(), &[])
            .unwrap()
            .diagnostics,
        [Diagnostic::LanguageMismatch]
    );
    result.checker.as_mut().unwrap().transcript.language_source = LanguageSource::Requested;
    assert!(
        result
            .classify(Options::default(), &[])
            .unwrap()
            .diagnostics
            .is_empty()
    );
}
