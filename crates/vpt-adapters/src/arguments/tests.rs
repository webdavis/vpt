use super::*;

#[test]
fn replacements_are_single_pass_and_unknown_tokens_stay_literal() {
    let expected = "{b}/$literal/{unknown}";
    let mut budget = expected.len();
    assert_eq!(
        replace(
            "{a}/{b}/{unknown}",
            &[("{a}", "{b}"), ("{b}", "$literal")],
            &mut budget
        ),
        Some(expected.into())
    );
    assert_eq!(budget, 0);
    assert!(replace("{a}", &[("{a}", "long")], &mut 3).is_none());
}

#[test]
fn one_budget_limits_expansion_across_multiple_arguments() {
    let mut budget = 7;
    assert_eq!(
        replace("{a}", &[("{a}", "four")], &mut budget),
        Some("four".into())
    );
    assert!(replace("{a}", &[("{a}", "four")], &mut budget).is_none());
}

#[test]
fn command_validation_bounds_count_bytes_and_executable_without_rejecting_empty_arguments() {
    assert!(valid(&["cmd".into(), "".into(), "abc".into()], 6));
    assert!(!valid(&["cmd".into(), "abcd".into()], 6));
    assert!(!valid(&[" ".into()], 8));
    assert!(!valid(&["cmd\0".into()], 8));
    assert!(!valid(&vec!["x".into(); 257], 1024));
}
