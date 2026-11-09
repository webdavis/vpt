use super::*;

#[test]
fn two_builtin_engines_supply_a_lexical_comparison() {
    let engine = Scripted::default();
    engine
        .outcomes
        .borrow_mut()
        .extend([Ok(output("apple")), Ok(output("whisply"))]);
    let result = transcribe(&pair(), &engine, &input()).unwrap();
    assert_eq!(
        result.comparison().unwrap().transcript.engine.name,
        "whisply"
    );
}

#[test]
fn generic_checker_is_retained_but_excluded_from_lexical_comparison() {
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
    settings.pair.checker = Some("custom".into());
    let engine = Scripted::default();
    engine
        .outcomes
        .borrow_mut()
        .extend([Ok(output("apple")), Ok(output("custom"))]);
    let result = transcribe(&settings, &engine, &input()).unwrap();
    assert!(result.lexical_classification);
    assert!(!result.single_engine());
    assert!(result.checker.is_some());
    assert!(result.comparison().is_none());
}
