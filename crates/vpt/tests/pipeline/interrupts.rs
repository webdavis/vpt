use super::*;

fn last_outcome(sandbox: &Sandbox, kind: &str) -> String {
    let entries: Vec<serde_json::Value> =
        fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    let run = entries
        .iter()
        .rev()
        .find(|entry| entry["fact"]["tag"] == "run_started" && entry["fact"]["kind"] == kind)
        .unwrap()["run_id"]
        .clone();
    entries
        .iter()
        .rev()
        .find(|entry| entry["run_id"] == run && entry["fact"]["tag"] == "run_finished")
        .unwrap()["fact"]["outcome"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn interrupted_analysis_marks_child_and_pipeline_runs_interrupted() {
    for pipeline in [false, true] {
        let sandbox = configured();
        add(&sandbox, "one.m4a", 1_787_604_456);
        json(&invoke(&sandbox, &[]));
        analysis::enable(&sandbox);
        let row = ledger(&sandbox).recordings().unwrap().remove(0);
        fs::write(
            sandbox.path().join("agent.sh"),
            "/bin/cat > /dev/null\nkill -INT \"$PPID\"\nexec /bin/sleep 10\n",
        )
        .unwrap();
        let output = if pipeline {
            invoke(&sandbox, &[])
        } else {
            support::run_with_budget(
                sandbox.vpt().args(["analyze", row.id.as_str(), "--json"]),
                std::time::Duration::from_secs(5),
            )
        };
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(stderr(&output).contains("interrupted"));
        assert_eq!(
            ledger(&sandbox)
                .by_id(&row.id)
                .unwrap()
                .unwrap()
                .stages
                .analysis,
            StageState::Failed
        );
        assert_eq!(last_outcome(&sandbox, "analyze"), "interrupted");
        if pipeline {
            assert_eq!(last_outcome(&sandbox, "run"), "interrupted");
        }
    }
}

#[test]
fn interrupted_transcription_also_marks_the_parent_pipeline_interrupted() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    fs::write(
        sandbox.path().join("bin/engine"),
        "#!/bin/sh\nkill -INT \"$PPID\"\nexec /bin/sleep 10\n",
    )
    .unwrap();
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(last_outcome(&sandbox, "transcribe"), "interrupted");
    assert_eq!(last_outcome(&sandbox, "run"), "interrupted");
}
