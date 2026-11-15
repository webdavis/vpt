use crate::fixture::{
    advice, assert_never_ran, check, configure, configured, detail, result, run_json, snapshot,
    tripwire,
};

const TOKEN: &str = "made-up-doctor-fixture-not-a-credential";

#[test]
fn without_a_token_speakers_are_one_not_configured_row() {
    let sandbox = configured("doctor-speakers-default");
    let document = run_json(&sandbox, 0);
    let speakers = check(&document, "speakers");
    assert_eq!(speakers["result"], "not_run");
    assert_eq!(speakers["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        detail(&document, "speakers", "token"),
        "not configured; set `engines.whisply.hf_token` in the config"
    );
}

#[test]
fn a_token_on_a_selected_whisply_labels_speakers_without_showing_the_token() {
    let sandbox = configured("doctor-speakers-on");
    let program = tripwire(&sandbox, "speaker-engine");
    configure(
        &sandbox,
        &format!(
            "[engines]\nchecker = 'whisply'\n[engines.whisply]\ncommand = [{program:?}]\nhf_token = {TOKEN:?}\n"
        ),
    );
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "speakers", "token"),
        "set on `engines.whisply`; whisply sends it to Hugging Face to fetch the speaker model"
    );
    assert_eq!(detail(&document, "speakers", "whisply"), "selected");
    assert_eq!(result(&document, "speakers", "test"), "not_run");
    assert_eq!(
        advice(&document, "speakers", "test"),
        "run `vpt test speakers`"
    );
    assert_eq!(check(&document, "speakers")["result"], "ok");
    assert_eq!(
        detail(&document, "engines", "whisply"),
        "selected, available, local"
    );
    assert!(!document.to_string().contains(TOKEN));
    assert_eq!(snapshot(&sandbox), before);
    assert_never_ran(&sandbox, "speaker-engine");
}

#[test]
fn a_token_on_an_unselected_whisply_is_a_warning_with_advice() {
    let sandbox = configured("doctor-speakers-unselected");
    configure(
        &sandbox,
        &format!("[engines.whisply]\ncommand = ['/bin/sh']\nhf_token = {TOKEN:?}\n"),
    );
    let document = run_json(&sandbox, 0);
    assert_eq!(result(&document, "speakers", "whisply"), "warning");
    assert_eq!(
        detail(&document, "speakers", "whisply"),
        "not selected, so the token gives no speaker labels"
    );
    assert_eq!(
        advice(&document, "speakers", "whisply"),
        "select it as `engines.main` or `engines.checker` in the config"
    );
    assert_eq!(
        detail(&document, "engines", "whisply"),
        "unused, available, local"
    );
}
