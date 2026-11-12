use super::*;

const ACTIONS: &str = r#"[
    {"text":"call the bank","sources":["00:00-00:01"],"owner":"me"},
    {"text":"book the room","sources":["00:00-00:01"],"owner":{"other":"Dana"}},
    {"text":"maybe follow up","sources":["00:00-00:01"]}
]"#;

fn labels(words: &str) -> std::process::Output {
    let (sandbox, record) = note::configured();
    let log = sandbox.path().join("sent");
    agent(
        &sandbox,
        &format!(
            "[actions]\ncommand = ['/bin/sh', '-c', 'printf \"%s \" \"$@\" >> {log:?}; echo >> {log:?}', 'todo-tool', '--label', '{{owner_word}}', '--', '{{text}}']\n{words}"
        ),
    );
    crate::actions::propose(&sandbox, ACTIONS);
    let mut output = invoke(&sandbox, record.id.as_str(), false);
    output.stdout = fs::read(&log).unwrap_or_default();
    output
}

fn sent(words: &str) -> String {
    let output = labels(words);
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn a_configured_word_reaches_the_command_and_an_unset_kind_falls_back_to_the_kind() {
    assert_eq!(
        sent("[actions.owner_words]\nme = 'mine'\nother = 'theirs'\n"),
        "--label mine -- call the bank \n--label theirs -- book the room \n--label unknown -- maybe follow up \n"
    );
    assert_eq!(
        sent(""),
        "--label me -- call the bank \n--label other -- book the room \n--label unknown -- maybe follow up \n"
    );
}

#[test]
fn an_empty_word_drops_the_label_flag_for_that_kind() {
    assert_eq!(
        sent("[actions.owner_words]\nother = ''\n"),
        "--label me -- call the bank \n-- book the room \n--label unknown -- maybe follow up \n"
    );
}

#[test]
fn an_owner_word_for_an_unknown_kind_refuses_the_configuration() {
    let output = labels("[actions.owner_words]\nboss = 'theirs'\n");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains("actions.owner_words.boss"),
        "{}",
        stderr(&output)
    );
}
