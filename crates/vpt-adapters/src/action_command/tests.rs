use super::expand::{OPTIONAL, expand};

fn run(command: &[&str], values: &[(&str, &str)]) -> Vec<String> {
    let command: Vec<String> = command.iter().map(|argument| (*argument).into()).collect();
    expand(&command, values, OPTIONAL).unwrap()
}

const NO_DUE: &[(&str, &str)] = &[
    ("{text}", "-call Dana"),
    ("{due}", ""),
    ("{owner_name}", ""),
];
const DUE: &[(&str, &str)] = &[("{text}", "-call Dana"), ("{due}", "2026-11-19T15:00")];

#[test]
fn a_kept_due_date_fills_both_arguments_and_an_absent_one_drops_the_pair() {
    let command = ["tool", "new", "--", "{text}", "--due", "{due}"];
    assert_eq!(
        run(&command, DUE),
        [
            "tool",
            "new",
            "--",
            "-call Dana",
            "--due",
            "2026-11-19T15:00"
        ]
    );
    assert_eq!(run(&command, NO_DUE), ["tool", "new", "--", "-call Dana"]);
}

#[test]
fn an_absent_value_never_takes_a_positional_argument_the_separator_or_the_program() {
    assert_eq!(run(&["tool", "new", "{due}"], NO_DUE), ["tool", "new"]);
    assert_eq!(run(&["tool", "{owner_name}"], NO_DUE), ["tool"]);
    assert_eq!(
        run(&["tool", "add", "{owner_name}"], NO_DUE),
        ["tool", "add"]
    );
    assert_eq!(run(&["tool", "--", "{owner_name}"], NO_DUE), ["tool", "--"]);
    assert_eq!(
        run(&["tool", "--text={text}", "{due}"], NO_DUE),
        ["tool", "--text=-call Dana"]
    );
}

#[test]
fn a_flag_joined_to_an_absent_value_is_dropped_whole_and_kept_when_present() {
    assert_eq!(run(&["tool", "--due={due}", "x"], NO_DUE), ["tool", "x"]);
    assert_eq!(
        run(&["tool", "--due={due}"], DUE),
        ["tool", "--due=2026-11-19T15:00"]
    );
    assert_eq!(run(&["tool", "Due: {due}"], NO_DUE), ["tool", "Due: "]);
}

#[test]
fn an_expansion_past_one_mebibyte_is_refused() {
    let text = "x".repeat(1_048_577);
    let command = ["todo".to_owned(), "{text}".to_owned()];
    assert!(expand(&command, &[("{text}", &text)], OPTIONAL).is_none());
}
