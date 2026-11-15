use crate::fixture::{
    advice, assert_never_ran, check, configure, configured, detail, issues, result, run_json,
    snapshot, tripwire,
};

#[test]
fn without_a_source_calendar_is_one_not_configured_row() {
    let sandbox = configured("doctor-calendar-default");
    let document = run_json(&sandbox, 0);
    let calendar = check(&document, "calendar");
    assert_eq!(calendar["result"], "not_run");
    assert_eq!(calendar["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        detail(&document, "calendar", "source"),
        "not configured; set `context.type` in the config"
    );
}

#[test]
fn a_command_source_is_found_and_never_run() {
    let sandbox = configured("doctor-calendar-command");
    let program = tripwire(&sandbox, "made-up-calendar");
    configure(
        &sandbox,
        &format!("[context]\ntype = 'dam'\n[context.dam]\ncommand = [{program:?}]\n"),
    );
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(detail(&document, "calendar", "source"), "dam");
    assert_eq!(
        detail(&document, "calendar", "command"),
        "configured, available; vpt asks it for events; whether it contacts a remote is up to that command"
    );
    assert_eq!(
        advice(&document, "calendar", "test"),
        "run `vpt test calendar`"
    );
    assert_eq!(snapshot(&sandbox), before);
    assert_never_ran(&sandbox, "made-up-calendar");
}

#[test]
fn a_missing_command_is_an_issue_with_advice() {
    let sandbox = configured("doctor-calendar-missing");
    configure(
        &sandbox,
        "[context]\ntype = 'dam'\n[context.dam]\ncommand = ['/absent/made-up-calendar']\n",
    );
    let document = run_json(&sandbox, 1);
    assert_eq!(
        advice(&document, "calendar", "command"),
        "make its program an executable file, or change `context.dam.command` in the config"
    );
    assert_eq!(issues(&document), ["calendar command"]);
}

#[test]
fn the_built_in_client_names_missing_credentials_and_never_shows_them() {
    let sandbox = configured("doctor-calendar-google");
    configure(
        &sandbox,
        "[context]\ntype = 'google'\n[context.google]\nclient_id = 'private-id'\n",
    );
    let document = run_json(&sandbox, 1);
    assert_eq!(
        detail(&document, "calendar", "source"),
        "google; vpt asks Google Calendar for events and sends it the credentials"
    );
    assert_eq!(result(&document, "calendar", "credentials"), "issue");
    assert_eq!(
        detail(&document, "calendar", "credentials"),
        "missing client_secret, refresh_token"
    );
    assert_eq!(
        advice(&document, "calendar", "credentials"),
        "set `context.google.client_secret` and `context.google.refresh_token` in the config"
    );
    assert!(!document.to_string().contains("private-"));
}

#[test]
fn complete_credentials_are_reported_as_set() {
    let sandbox = configured("doctor-calendar-google-set");
    configure(
        &sandbox,
        "[context]\ntype = 'google'\n[context.google]\nclient_id = 'private-id'\nclient_secret = 'private-secret'\nrefresh_token = 'private-token'\n",
    );
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "calendar", "credentials"),
        "client_id, client_secret and refresh_token set"
    );
    assert_eq!(result(&document, "calendar", "test"), "not_run");
    assert!(!document.to_string().contains("private-"));
}
