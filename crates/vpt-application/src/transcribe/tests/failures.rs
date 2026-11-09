use super::*;

#[test]
fn checker_failure_preserves_main_output_with_single_engine_diagnostic() {
    let engine = Scripted::default();
    engine
        .outcomes
        .borrow_mut()
        .extend([Ok(output("apple")), Err(EngineFailure::InvalidDocument)]);
    let result = transcribe(&pair(), &engine, &input()).unwrap();
    assert_eq!(result.record_slot, EngineSlot::Main);
    assert_eq!(result.record.raw_document, b"private-canary");
    assert!(result.single_engine());
    assert_eq!(result.failures[0].slot, EngineSlot::Checker);
}

#[test]
fn two_failures_return_both_safe_causes_without_an_accepted_transcript() {
    let engine = Scripted::default();
    engine.outcomes.borrow_mut().extend([
        Err(EngineFailure::Exited(7)),
        Err(EngineFailure::InvalidDocument),
    ]);
    let error = transcribe(&pair(), &engine, &input()).unwrap_err();
    let TranscriptionError::Failed(failures) = error else {
        panic!("wrong failure: {error:?}");
    };
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0].cause, EngineFailure::Exited(7));
    assert_eq!(failures[1].cause, EngineFailure::InvalidDocument);
}

#[test]
fn same_family_and_invalid_duration_fail_before_an_engine_is_called() {
    let mut settings = pair();
    settings.pair.checker = Some("apple".into());
    let engine = Scripted::default();
    assert!(matches!(
        transcribe(&settings, &engine, &input()),
        Err(TranscriptionError::Selection(_))
    ));
    for duration in [f64::NAN, f64::INFINITY, -0.1] {
        let mut input = input();
        input.duration = duration;
        assert_eq!(
            transcribe(&pair(), &engine, &input).unwrap_err(),
            TranscriptionError::InvalidAudioDuration
        );
    }
    assert!(engine.calls.borrow().is_empty());
}

#[test]
fn interrupt_stops_before_checker_and_is_not_a_fallback_trigger() {
    let engine = Scripted::default();
    engine
        .outcomes
        .borrow_mut()
        .push_back(Err(EngineFailure::Interrupted));
    assert!(matches!(
        transcribe(&pair(), &engine, &input()),
        Err(TranscriptionError::Interrupted(_))
    ));
    assert_eq!(engine.calls.borrow().len(), 1);
}

#[test]
fn no_checker_is_a_successful_single_engine_run() {
    let engine = Scripted::default();
    engine.outcomes.borrow_mut().push_back(Ok(output("apple")));
    let result = transcribe(&EngineSettings::default(), &engine, &input()).unwrap();
    assert!(result.single_engine());
    assert_eq!(result.record_slot, EngineSlot::Main);
    assert_eq!(engine.calls.borrow().len(), 1);
}
