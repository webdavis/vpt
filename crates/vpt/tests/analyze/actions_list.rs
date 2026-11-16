use super::*;
use crate::actions::{configure_with, spoken};

fn list(sandbox: &Sandbox, extra: &[&str]) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["actions", "list"]).args(extra),
        Duration::from_secs(5),
    )
}

fn states(document: &serde_json::Value) -> Vec<(String, String)> {
    document["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["state"].as_str().unwrap().to_owned(),
                row["text"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn each_follow_up_is_listed_with_its_state_and_listing_writes_nothing() {
    let (sandbox, record) = spoken();
    let log = configure_with(&sandbox, "final_exit_codes = [4]\n");
    fs::write(log.with_extension("fail"), "4").unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let before = fixture::snapshot(&sandbox);
    let all = note::success(&list(&sandbox, &["-o", "json"]));
    assert_eq!(all["command"], "actions list");
    let rows = states(&all);
    assert_eq!(rows.len(), 4, "{all}");
    assert!(rows.iter().all(|(state, _)| state == "refused"), "{all}");
    assert!(
        rows.iter()
            .any(|(_, text)| text == "- send the secret invoice")
    );
    assert!(
        all["actions"][0]["id"]
            .as_str()
            .unwrap()
            .starts_with(record.id.as_str())
    );
    assert_eq!(all["actions"][0]["recording"], record.id.as_str());
    let one = note::success(&list(&sandbox, &[record.id.as_str(), "--output", "json"]));
    assert_eq!(one["actions"], all["actions"]);
    let human = stdout(&list(&sandbox, &[]));
    assert!(human.contains(record.id.as_str()), "{human}");
    assert!(
        human.contains("✗ refused: - send the secret invoice"),
        "{human}"
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn sent_follow_ups_are_listed_as_sent() {
    let (sandbox, record) = spoken();
    configure_with(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let rows = states(&note::success(&list(&sandbox, &["-o", "json"])));
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|(state, _)| state == "sent"), "{rows:?}");
}

#[test]
fn an_empty_list_exits_zero_and_says_so() {
    let (sandbox, _) = spoken();
    configure_with(&sandbox, "");
    let document = note::success(&list(&sandbox, &["-o", "json"]));
    assert_eq!(document["actions"], serde_json::json!([]));
    let output = list(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "no follow-ups\n");
}

#[test]
fn an_invalid_recording_id_is_a_usage_error() {
    let (sandbox, _) = spoken();
    let output = list(&sandbox, &["not-an-id"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("invalid recording id"));
}
