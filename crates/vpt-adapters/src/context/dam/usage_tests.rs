use super::collection_tests::{fixture, request};
use super::*;

const OLDER: &str = "dam rejected the command line; it lacks a subcommand or flag that vpt passes, so it needs a newer dam";
const OTHER: &str =
    "dam rejected the command line that vpt sent, and not for a missing subcommand or flag";

fn calendar_detail(stderr: &str, max_age: &[String]) -> String {
    let (_dir, command) = fixture(&format!(
        "case \"$2\" in\nagenda) printf '%s' \"{stderr}\" >&2; exit 2;;\n*) printf '%s' '{{\"objects\":[]}}';;\nesac"
    ));
    let snapshot = DamContext {
        command: &command,
        max_age,
    }
    .collect(&request(&["calendar/".into()], &["work/".into()]))
    .unwrap();
    snapshot.status.calendar.detail().unwrap().to_owned()
}

fn document(problem: &str, word: &str) -> String {
    format!(
        r#"{{\"error\":{{\"kind\":\"usage\",\"rule\":null,\"message\":\"PRIVATE\",\"oids\":[],\"usage\":{{\"problem\":\"{problem}\",\"word\":{word}}}}}}}"#
    )
}

#[test]
fn a_dam_that_lacks_a_subcommand_or_flag_vpt_passed_needs_a_newer_dam() {
    let max_age = ["gcal=15m".to_owned()];
    for stderr in [
        document("unknown_subcommand", r#"\"agenda\""#),
        document("unknown_flag", r#"\"--max-age\""#),
        "error: unrecognized subcommand 'agenda'".to_owned(),
        "error: unexpected argument '--max-age' found".to_owned(),
    ] {
        assert_eq!(calendar_detail(&stderr, &max_age), OLDER, "{stderr}");
    }
}

#[test]
fn any_other_usage_error_is_not_blamed_on_the_dam_version() {
    for stderr in [
        document("bad_value", r#"\"--from\""#),
        document("unknown_flag", r#"\"--bogus\""#),
        document("unknown_flag", "null"),
        document("other", "null"),
        "error: invalid value for '--from <FROM>'".to_owned(),
        "error: unexpected argument '--bogus' found".to_owned(),
        String::new(),
    ] {
        assert_eq!(calendar_detail(&stderr, &[]), OTHER, "{stderr}");
    }
}
