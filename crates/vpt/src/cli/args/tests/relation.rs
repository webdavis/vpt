use super::*;

#[test]
fn relation_confirmation_keeps_private_target_text_out_of_debug() {
    let invocation = parsed("confirm memo --relation related:Private-target --json").unwrap();
    assert!(!format!("{invocation:?}").contains("Private-target"));
    let Verb::ConfirmRelation { id, relation } = invocation.verb else {
        panic!("relation")
    };
    assert_eq!(id, "memo");
    assert_eq!(relation.as_str(), "related:Private-target");
}

#[test]
fn relation_confirmation_requires_one_action_and_one_recording() {
    for input in [
        "confirm --relation related:Private",
        "confirm memo --relation",
        "confirm memo extra --relation related:Private",
        "confirm memo --relation related:Private --tag Private",
        "confirm memo --relation related:Private --term Private",
        "confirm memo --relation related:Private --relation related:Private",
        "show memo --relation related:Private",
    ] {
        let error = parsed(input).unwrap_err();
        assert!(!error.0.contains("Private"));
    }
}
