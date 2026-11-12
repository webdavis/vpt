use super::*;

#[test]
fn progress_lines_bracket_the_agent_command_on_stderr() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let output = support::run_with_budget(
        sandbox
            .vpt()
            .args(["synthesize", record.id.as_str(), "--json", "--progress"]),
        Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let result: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(result["command"], "synthesize");
    let states: Vec<serde_json::Value> = stderr(&output)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .map(|line| {
            serde_json::json!([
                line["schema"],
                line["recording"],
                line["step"],
                line["state"]
            ])
        })
        .collect();
    let id = record.id.as_str();
    assert_eq!(
        states,
        [
            serde_json::json!(["vpt.progress/1", id, "synthesize", "started"]),
            serde_json::json!(["vpt.progress/1", id, "synthesize", "done"]),
        ]
    );
}
