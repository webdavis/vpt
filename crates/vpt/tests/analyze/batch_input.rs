use super::batch::{bulk_agent, calls, propose};
use super::*;
use serde_json::{Value, json};

fn piped(sandbox: &Sandbox, input: &str, extra: &[&str]) -> std::process::Output {
    let path = sandbox.path().join("stdin.txt");
    fs::write(&path, input).unwrap();
    support::run_with_budget(
        sandbox
            .vpt()
            .args(["analyze", "--stdin", "--json"])
            .args(extra)
            .stdin(fs::File::open(path).unwrap()),
        Duration::from_secs(20),
    )
}

#[test]
fn stdin_runs_the_listed_ids_and_reports_bad_ones_as_their_failures() {
    let (sandbox, rows) = note::configured_with(&[7, 8]);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &[]);
    let unknown = fixture::record(&sandbox, 42, StageStates::fresh()).id;
    let input = format!(
        "\n{}\n\nnot-an-id\n  {unknown}  \n{}\n",
        rows[1].id, rows[0].id
    );
    let output = piped(&sandbox, &input, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["ids"],
        json!(["not-an-id", unknown.as_str()])
    );
    assert_eq!(
        error["error"]["completed"],
        json!([rows[1].id.as_str(), rows[0].id.as_str()])
    );
    assert_eq!(calls(&sandbox), [rows[1].id.as_str(), rows[0].id.as_str()]);
}

#[test]
fn stdin_with_only_good_ids_succeeds_in_the_given_order() {
    let (sandbox, rows) = note::configured_with(&[7, 8]);
    bulk_agent(&sandbox);
    propose(&sandbox, "words", &[]);
    let input = format!("{}\n{}\n", rows[1].id, rows[0].id);
    let result = note::success(&piped(&sandbox, &input, &[]));
    assert_eq!(result["recordings"][0]["id"], rows[1].id.as_str());
    assert_eq!(result["recordings"][1]["id"], rows[0].id.as_str());
    let planned = note::success(&piped(&sandbox, &input, &["--dry-run"]));
    assert_eq!(planned["recordings"][0]["status"], "planned");
    assert_eq!(calls(&sandbox).len(), 2);
}
