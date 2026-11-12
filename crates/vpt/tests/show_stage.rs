#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_stage/actions.rs"]
mod actions;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "redact/fixture.rs"]
mod recording;
mod support;
use serde_json::json;
use std::fs;
use support::{Sandbox, stderr};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;

fn invoke(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(sandbox.vpt().args(args), std::time::Duration::from_secs(5))
}
fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}
fn configured(name: &str) -> (Sandbox, vpt_domain::identity::RecordingId) {
    let sandbox = fixture::configured(name);
    let id = recording::seed_flags(&sandbox, true);
    (sandbox, id)
}
fn show(sandbox: &Sandbox, id: &str, stage: &str) -> std::process::Output {
    invoke(sandbox, &["show", id, "--stage", stage, "--json"])
}
fn correct_the_flag(sandbox: &Sandbox, id: &vpt_domain::identity::RecordingId, text: &str) {
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let flag = ledger
        .review_snapshot(id)
        .unwrap()
        .unwrap()
        .flags
        .into_iter()
        .find(|f| matches!(&f.content, vpt_domain::review::FlagContent::Lexical(_)))
        .unwrap();
    let flag = flag.identifier.hex();
    json(&invoke(
        sandbox,
        &[
            "review",
            id.as_str(),
            "--resolve",
            &flag,
            "--correct",
            text,
            "--json",
        ],
    ));
}

#[test]
fn transcript_segments_carry_corrections_without_an_opt_in() {
    let (sandbox, id) = configured("show-segments");
    correct_the_flag(&sandbox, &id, "5678");
    let before = fixture::snapshot(&sandbox);
    let result = json(&show(&sandbox, id.as_str(), "transcript"));
    assert_eq!(result["schema"], "vpt.result/1");
    assert_eq!(result["command"], "show");
    assert_eq!(result["recording"], id.as_str());
    assert_eq!(result["stage"], "transcript");
    assert_eq!(
        result["segments"],
        json!([{
            "start": 0.0,
            "end": 1.0,
            "timecode": "00:00",
            "text": "Meet Alice with Bob via person@sample.test for 5678",
        }])
    );
    assert!(
        result["content"]
            .as_str()
            .unwrap()
            .contains("Meet Alice with Bob")
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn a_stage_without_saved_state_is_an_input_error() {
    let sandbox = fixture::configured("show-stage-missing");
    let id = fixture::record(&sandbox, 3, StageStates::fresh()).id;
    let unknown = show(&sandbox, id.as_str(), "transcript");
    assert_eq!(unknown.status.code(), Some(2), "{}", stderr(&unknown));
    assert!(!stderr(&unknown).contains("unexpected argument"));
    let wrong = invoke(
        &sandbox,
        &["show", id.as_str(), "--stage", "audio", "--json"],
    );
    assert_eq!(wrong.status.code(), Some(2), "{}", stderr(&wrong));
}

#[test]
fn analysis_lists_summary_and_actions_with_a_moment_per_line() {
    let (sandbox, id) = configured("show-analysis");
    let script = sandbox.path().join("agent.sh");
    let proposal = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    fs::write(&proposal, r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Alice meets Bob","sources":["00:00-00:01"]}],"actions":[{"text":"Email Bob","sources":["00:00-00:01"]},{"text":"Call Alice","sources":[]}],"tags":[],"relations":[]}"#).unwrap();
    fixture::write_config(
        &sandbox,
        &format!("[analysis]\ncommand=['/bin/sh', {script:?}, {proposal:?}]\n"),
    );
    json(&invoke(&sandbox, &["analyze", id.as_str(), "--json"]));
    let before = fixture::snapshot(&sandbox);
    let result = json(&show(&sandbox, id.as_str(), "analysis"));
    let moment = format!("{}@00:00", id.as_str());
    let source = json!([{"start": 0, "end": 1, "range": "00:00-00:01"}]);
    assert_eq!(result["recording"], id.as_str());
    assert_eq!(result["stage"], "analysis");
    assert_eq!(result["summary"][0]["moment"], moment.as_str());
    assert_eq!(result["summary"][0]["text"], "Alice meets Bob");
    assert_eq!(result["summary"][0]["sources"], source);
    assert_eq!(result["actions"][0]["moment"], moment.as_str());
    assert_eq!(result["actions"][0]["text"], "Email Bob");
    assert_eq!(result["actions"][0]["sources"], source);
    assert_eq!(result["actions"][1]["moment"], json!(null));
    assert_eq!(result["actions"][1]["finding"], "unsourced");
    assert_eq!(result["actions"].as_array().unwrap().len(), 2);
    assert_eq!(fixture::snapshot(&sandbox), before);
    let mut again = json(&show(&sandbox, id.as_str(), "analysis"));
    again["generated_at"] = result["generated_at"].clone();
    assert_eq!(again, result);
}

#[test]
fn analysis_without_an_accepted_proposal_is_an_input_error() {
    let (sandbox, id) = configured("show-analysis-missing");
    let missing = show(&sandbox, id.as_str(), "analysis");
    assert_eq!(missing.status.code(), Some(2), "{}", stderr(&missing));
    assert!(missing.stdout.is_empty());
}

#[path = "show_stage/boundaries.rs"]
mod boundaries;
#[path = "show_stage/copies.rs"]
mod copies;
#[path = "show_stage/due_words.rs"]
mod due_words;
#[path = "show_stage/redact.rs"]
mod redact;
#[path = "show_stage/text.rs"]
mod text;
