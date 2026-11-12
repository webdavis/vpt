#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod record;
#[path = "search/fixture.rs"]
mod seed;
mod support;
use serde_json::{Value, json};
use support::{Sandbox, stderr, stdout};

fn invoke(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(sandbox.vpt().args(args), std::time::Duration::from_secs(5))
}

fn json(output: &std::process::Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn refused(output: &std::process::Output, code: i32) -> Value {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn corpus(name: &str) -> (Sandbox, String, String) {
    let sandbox = record::configured(name);
    let older = seed::transcribed(
        &sandbox,
        1,
        &[
            (0.0, "Welcome everyone"),
            (250.4, "The budget REVIEW moves to Friday"),
            (4510.0, "Budget is final at 5:30 says person@sample.test"),
        ],
    );
    seed::analyzed(
        &sandbox,
        &older,
        r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"The budget review moves to Friday","sources":["04:10-04:14"]}],"actions":[],"tags":[],"relations":[]}"#,
    );
    let newer = seed::transcribed(&sandbox, 2, &[(5.0, "Review the budget before lunch")]);
    (sandbox, older.as_str().into(), newer.as_str().into())
}

fn hits(sandbox: &Sandbox, args: &[&str]) -> Vec<Value> {
    let mut command = vec!["--json", "search"];
    command.extend(args);
    json(&invoke(sandbox, &command))["hits"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn search_returns_every_line_holding_all_words_with_its_moment() {
    let (sandbox, older, newer) = corpus("search-moments");
    let before = record::snapshot(&sandbox);
    let result = json(&invoke(&sandbox, &["--json", "search", "budget", "review"]));
    assert_eq!(result["schema"], "vpt.result/1");
    assert_eq!(result["command"], "search");
    assert_eq!(result["words"], json!(["budget", "review"]));
    assert_eq!(result["limit"], 50);
    assert_eq!(result["truncated"], false);
    assert_eq!(
        result["hits"],
        json!([
            {
                "moment": format!("{newer}@00:05"),
                "recording": newer,
                "title": "title-2",
                "stage": "transcript",
                "seconds": 5,
                "timecode": "00:05",
                "quote": "Review the budget before lunch",
            },
            {
                "moment": format!("{older}@04:10"),
                "recording": older,
                "title": "title-1",
                "stage": "transcript",
                "seconds": 250,
                "timecode": "04:10",
                "quote": "The budget REVIEW moves to Friday",
            },
            {
                "moment": format!("{older}@04:10"),
                "recording": older,
                "title": "title-1",
                "stage": "analysis",
                "seconds": 250,
                "timecode": "04:10",
                "quote": "The budget review moves to Friday",
            },
        ])
    );
    assert_eq!(record::snapshot(&sandbox), before);
}

#[test]
fn search_matches_the_spoken_text_rather_than_its_markdown_escapes() {
    let (sandbox, older, _) = corpus("search-escapes");
    let found = hits(&sandbox, &["5:30", "PERSON@sample"]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["moment"], format!("{older}@75:10"));
    assert_eq!(
        found[0]["quote"],
        "Budget is final at 5:30 says person@sample.test"
    );
}

#[test]
fn search_prints_one_human_line_per_hit() {
    let (sandbox, older, _) = corpus("search-human");
    let output = invoke(&sandbox, &["search", "friday"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!(
            "{older}@04:10  title-1  transcript  The budget REVIEW moves to Friday\n\
             {older}@04:10  title-1  analysis  The budget review moves to Friday\n"
        )
    );
    let none = invoke(&sandbox, &["search", "absent"]);
    assert_eq!(stdout(&none), "no matches\n");
}

#[test]
fn search_bounds_its_hits_and_says_when_it_cut_them() {
    let (sandbox, _, newer) = corpus("search-limit");
    let result = json(&invoke(
        &sandbox,
        &["--json", "search", "budget", "--limit", "1"],
    ));
    assert_eq!(result["limit"], 1);
    assert_eq!(result["truncated"], true);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    assert_eq!(result["hits"][0]["recording"], newer);
    for args in [
        &["--json", "search"][..],
        &["--json", "search", "budget", "--limit", "0"],
        &["--json", "search", "budget", "--limit", "many"],
    ] {
        assert_eq!(
            refused(&invoke(&sandbox, args), 2)["error"]["kind"],
            "usage"
        );
    }
}

#[test]
fn search_skips_a_note_that_is_no_longer_on_disk() {
    let (sandbox, older, _) = corpus("search-removed");
    let ledger = vpt_adapters::SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let id = vpt_domain::identity::RecordingId::parse(&older).unwrap();
    let note = vpt_application::ports::NoteLedger::note(
        &ledger,
        &id,
        vpt_application::ports::NoteStage::Transcript,
    )
    .unwrap()
    .unwrap();
    std::fs::remove_file(note.target).unwrap();
    let found = hits(&sandbox, &["friday"]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["stage"], "analysis");
}

#[test]
fn search_without_a_ledger_finds_nothing_and_creates_nothing() {
    let sandbox = record::configured("search-absent");
    let before = record::snapshot(&sandbox);
    assert_eq!(hits(&sandbox, &["budget"]), Vec::<Value>::new());
    assert_eq!(record::snapshot(&sandbox), before);
}
