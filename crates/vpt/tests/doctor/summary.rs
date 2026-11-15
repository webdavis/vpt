use crate::fixture::{
    advice, assert_never_ran, check, configure, configured, detail, issues, result, run_json,
    snapshot, tripwire,
};

#[test]
fn without_a_command_summary_is_one_not_configured_row() {
    let sandbox = configured("doctor-summary-default");
    let document = run_json(&sandbox, 0);
    let summary = check(&document, "summary");
    assert_eq!(summary["result"], "not_run");
    assert_eq!(summary["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        detail(&document, "summary", "command"),
        "not configured; set `analysis.command` in the config"
    );
}

#[test]
fn a_configured_command_is_found_never_run_and_says_where_the_transcript_goes() {
    let sandbox = configured("doctor-summary-found");
    let program = tripwire(&sandbox, "made-up-agent");
    configure(
        &sandbox,
        &format!("[analysis]\ncommand = [{program:?}, 'private-argument']\n"),
    );
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "summary", "command"),
        "configured, available; it receives the full transcript, and whatever it calls sees it"
    );
    assert_eq!(result(&document, "summary", "test"), "not_run");
    assert_eq!(
        advice(&document, "summary", "test"),
        "run `vpt test summary`"
    );
    assert!(!document.to_string().contains("private-"));
    assert_eq!(snapshot(&sandbox), before);
    assert_never_ran(&sandbox, "made-up-agent");
}

#[test]
fn a_missing_program_is_an_issue_with_advice() {
    let sandbox = configured("doctor-summary-missing");
    configure(
        &sandbox,
        "[analysis]\ncommand = ['/absent/made-up-agent']\n",
    );
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "summary", "command"), "issue");
    assert_eq!(
        advice(&document, "summary", "command"),
        "make its program an executable file, or change `analysis.command` in the config"
    );
    assert_eq!(issues(&document), ["summary command"]);
}
