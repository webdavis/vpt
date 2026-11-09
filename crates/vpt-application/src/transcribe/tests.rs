mod comparison;
mod failures;
use super::*;
use crate::engines::{CheckerRuns, EngineConfig, EngineKind};
use crate::ports::{EngineFailure, EngineOutput, EngineRequest};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;
use vpt_domain::transcript::{EngineMetadata, Transcript};

#[derive(Default)]
struct Scripted {
    calls: RefCell<Vec<(String, String, Duration)>>,
    outcomes: RefCell<VecDeque<Result<EngineOutput, EngineFailure>>>,
}

impl Engine for Scripted {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
        self.calls.borrow_mut().push((
            request.engine.name.clone(),
            request.language.into(),
            request.timeout,
        ));
        self.outcomes
            .borrow_mut()
            .pop_front()
            .expect("unexpected engine call")
    }
}

fn output(name: &str) -> EngineOutput {
    EngineOutput {
        transcript: Transcript {
            language_source: vpt_domain::transcript::LanguageSource::Reported,
            engine: EngineMetadata {
                name: name.into(),
                tool: "fixture".into(),
                version: "1".into(),
                model: "fixture".into(),
                family: name.into(),
            },
            language: "en-US".into(),
            local: true,
            segments: vec![],
        },
        raw_document: b"private-canary".to_vec(),
        diagnostics: vec![],
    }
}

fn input() -> TranscriptionInput<'static> {
    TranscriptionInput {
        audio: Path::new("/archive/audio.m4a"),
        out_dir: Path::new("/outputs"),
        language: Some("fr-CA"),
        duration: 1.5,
    }
}

fn pair() -> EngineSettings {
    let mut settings = EngineSettings::default();
    settings.pair.checker = Some("whisply".into());
    settings
}

#[test]
fn always_runs_main_then_checker_and_keeps_main_as_record() {
    let engine = Scripted::default();
    engine
        .outcomes
        .borrow_mut()
        .extend([Ok(output("apple")), Ok(output("whisply"))]);
    let result = transcribe(&pair(), &engine, &input()).unwrap();
    assert_eq!(result.record_slot, EngineSlot::Main);
    assert_eq!(result.record.transcript.engine.name, "apple");
    assert_eq!(result.checker.unwrap().transcript.engine.name, "whisply");
    assert!(result.lexical_classification);
    assert_eq!(result.language, "fr-CA");
    assert_eq!(
        *engine.calls.borrow(),
        [
            ("apple".into(), "fr-CA".into(), Duration::from_secs(1800)),
            ("whisply".into(), "fr-CA".into(), Duration::from_secs(1800))
        ]
    );
}

#[test]
fn on_failure_skips_checker_after_main_success() {
    let mut settings = pair();
    settings.checker_runs = CheckerRuns::OnFailure;
    let engine = Scripted::default();
    engine.outcomes.borrow_mut().push_back(Ok(output("apple")));
    let result = transcribe(&settings, &engine, &input()).unwrap();
    assert_eq!(engine.calls.borrow().len(), 1);
    assert!(result.single_engine());
    assert!(result.failures.is_empty());
}

#[test]
fn checker_becomes_record_when_main_fails_under_either_policy() {
    for policy in [CheckerRuns::Always, CheckerRuns::OnFailure] {
        let mut settings = pair();
        settings.checker_runs = policy;
        let engine = Scripted::default();
        engine
            .outcomes
            .borrow_mut()
            .extend([Err(EngineFailure::DeadlineExceeded), Ok(output("whisply"))]);
        let result = transcribe(&settings, &engine, &input()).unwrap();
        assert_eq!(result.record_slot, EngineSlot::Checker);
        assert_eq!(result.record.transcript.engine.name, "whisply");
        assert!(result.single_engine());
        assert_eq!(
            result.failures,
            [AttemptFailure {
                engine: "apple".into(),
                slot: EngineSlot::Main,
                cause: EngineFailure::DeadlineExceeded
            }]
        );
    }
}

#[test]
fn generic_record_is_exempt_from_lexical_classification() {
    let mut settings = pair();
    settings.definitions.insert(
        "custom".into(),
        EngineConfig {
            name: "custom".into(),
            kind: EngineKind::Command {
                command: vec!["engine".into()],
                family: "custom".into(),
                local: false,
            },
        },
    );
    settings.pair.main = "custom".into();
    let engine = Scripted::default();
    engine
        .outcomes
        .borrow_mut()
        .extend([Ok(output("custom")), Ok(output("whisply"))]);
    let result = transcribe(&settings, &engine, &input()).unwrap();
    assert!(!result.lexical_classification);
    assert!(!result.single_engine());
}
