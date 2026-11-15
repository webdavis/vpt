use crate::fixture::{
    advice, assert_never_ran, check, configure, configured, detail, issues, result, run_json,
    snapshot, tripwire,
};

#[test]
fn without_a_command_actions_are_one_not_configured_row() {
    let sandbox = configured("doctor-actions-default");
    let document = run_json(&sandbox, 0);
    let actions = check(&document, "actions");
    assert_eq!(actions["result"], "not_run");
    assert_eq!(actions["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        detail(&document, "actions", "command"),
        "not configured; set `actions.command` in the config"
    );
}

#[test]
fn both_commands_are_found_never_run_and_env_values_stay_hidden() {
    let sandbox = configured("doctor-actions-found");
    let send = tripwire(&sandbox, "made-up-send");
    let withdraw = tripwire(&sandbox, "made-up-withdraw");
    configure(
        &sandbox,
        &format!(
            "[actions]\ncommand = [{send:?}, '{{text}}']\nwithdraw_command = [{withdraw:?}, '{{id}}']\n[actions.env]\nMADE_UP_KEY = 'private-value'\nOTHER_KEY = 'private-other'\n"
        ),
    );
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "actions", "command"),
        "configured, available; it sends each follow-up to whatever it calls"
    );
    assert_eq!(
        detail(&document, "actions", "withdraw_command"),
        "configured, available; it sends each withdrawal to whatever it calls"
    );
    assert_eq!(
        detail(&document, "actions", "env"),
        "MADE_UP_KEY, OTHER_KEY set for both commands"
    );
    assert_eq!(result(&document, "actions", "test"), "not_run");
    assert_eq!(
        advice(&document, "actions", "test"),
        "run `vpt test actions`"
    );
    assert!(!document.to_string().contains("private-"));
    assert_eq!(snapshot(&sandbox), before);
    assert_never_ran(&sandbox, "made-up-send");
    assert_never_ran(&sandbox, "made-up-withdraw");
}

#[test]
fn without_a_withdraw_command_the_test_advice_asks_for_one_first() {
    let sandbox = configured("doctor-actions-no-withdraw");
    configure(&sandbox, "[actions]\ncommand = ['/bin/sh']\n");
    let document = run_json(&sandbox, 0);
    assert_eq!(result(&document, "actions", "withdraw_command"), "not_run");
    assert_eq!(
        detail(&document, "actions", "withdraw_command"),
        "not configured; set `actions.withdraw_command` in the config"
    );
    assert_eq!(detail(&document, "actions", "env"), "none set");
    assert_eq!(
        advice(&document, "actions", "test"),
        "set `actions.withdraw_command`, then run `vpt test actions`"
    );
    assert_eq!(check(&document, "actions")["result"], "ok");
}

#[test]
fn a_missing_program_is_an_issue_with_advice() {
    let sandbox = configured("doctor-actions-missing");
    configure(
        &sandbox,
        "[actions]\ncommand = ['/bin/sh']\nwithdraw_command = ['/absent/made-up-withdraw']\n",
    );
    let document = run_json(&sandbox, 1);
    assert_eq!(
        advice(&document, "actions", "withdraw_command"),
        "make its program an executable file, or change `actions.withdraw_command` in the config"
    );
    assert_eq!(issues(&document), ["actions withdraw_command"]);
}
