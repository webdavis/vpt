use super::*;

#[test]
fn relation_confirmation_keeps_private_target_text_out_of_debug() {
    for relation in ["related:Private-target", "-related:Private"] {
        let invocation =
            parse(["vpt", "confirm", "relation", "memo", relation, "--json"].map(OsString::from))
                .unwrap();
        assert!(!format!("{invocation:?}").contains(relation));
        let Verb::ConfirmRelation {
            id,
            relation: parsed,
        } = invocation.verb
        else {
            panic!("relation")
        };
        assert_eq!((id.as_str(), parsed.as_str()), ("memo", relation));
    }
}

#[test]
fn relation_confirmation_requires_one_recording_and_one_relation() {
    for input in [
        "confirm relation related:Private",
        "confirm relation memo",
        "confirm relation memo related:Private extra",
        "confirm relation memo related:Private --tag Private",
        "show memo --relation related:Private",
    ] {
        let error = parsed(input).unwrap_err();
        assert!(!format!("{error:?}").contains("Private"), "{input}");
    }
}
