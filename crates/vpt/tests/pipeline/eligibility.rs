use super::*;

fn transitions(sandbox: &Sandbox) -> Vec<serde_json::Value> {
    fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter_map(|entry| {
            (entry["fact"]["tag"] == "synthesis_state_changed").then(|| entry["fact"].clone())
        })
        .collect()
}

#[test]
fn failed_stage_history_rolls_back_state_and_stops_new_work() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let history = transitions(&sandbox);
    sql(
        &sandbox,
        "UPDATE recordings SET stage_synthesis='pending'; CREATE TRIGGER refuse_stage_history BEFORE INSERT ON history_records WHEN NEW.fact LIKE '%synthesis_state_changed%' BEGIN SELECT RAISE(ABORT, 'Private transition failure'); END",
    );
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!stderr(&output).contains("Private transition failure"));
    assert!(output.stdout.is_empty());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Pending
    );
    assert_eq!(transitions(&sandbox), history);
    assert_eq!(synthesis::calls(&sandbox, "calls"), 1);
    sql(&sandbox, "DROP TRIGGER refuse_stage_history");
    json(&invoke(&sandbox, &[]));
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Disabled
    );
}

#[test]
fn changed_input_is_pending_even_when_note_refusal_prevents_the_agent() {
    let sandbox = configured();
    synthesis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let proposal = ledger(&sandbox).accepted_proposal(&row.id).unwrap();
    let source = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let edited = source.text.replace("<!-- vpt:content end -->", "");
    fs::write(&source.target, &edited).unwrap();
    synthesis::append(&sandbox, "\n[readability]\nenabled=true\n");
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Pending
    );
    assert_eq!(fs::read_to_string(&source.target).unwrap(), edited);
    assert_eq!(
        ledger(&sandbox).accepted_proposal(&row.id).unwrap(),
        proposal
    );
    assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
    assert_eq!(transitions(&sandbox).last().unwrap()["from"], "succeeded");
    assert_eq!(transitions(&sandbox).last().unwrap()["to"], "pending");
}

#[test]
fn disabled_synthesis_is_saved_once_and_enabling_it_is_logged_before_work() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    assert_eq!(row.stages.synthesis, StageState::Disabled);
    let changed = transitions(&sandbox);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0]["from"], "pending");
    assert_eq!(changed[0]["to"], "disabled");
    json(&invoke(&sandbox, &[]));
    assert_eq!(transitions(&sandbox), changed);
    synthesis::enable(&sandbox);
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"][0]["stage"],
        "synthesize"
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Disabled
    );
    json(&invoke(&sandbox, &[]));
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Succeeded
    );
    let changed = transitions(&sandbox);
    assert_eq!(changed.len(), 2);
    assert_eq!(changed[1]["from"], "disabled");
    assert_eq!(changed[1]["to"], "pending");
    assert_eq!(synthesis::calls(&sandbox, "calls"), 1);
    assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn disabling_preserves_success_and_expiration_but_disables_failed_work() {
    for initial in [
        StageState::Succeeded,
        StageState::Expired,
        StageState::Failed,
    ] {
        let sandbox = configured();
        synthesis::enable(&sandbox);
        add(&sandbox, "one.m4a", 1_787_604_456);
        json(&invoke(&sandbox, &[]));
        let row = ledger(&sandbox).recordings().unwrap().remove(0);
        let proposal = ledger(&sandbox).accepted_proposal(&row.id).unwrap();
        sql(
            &sandbox,
            &format!(
                "UPDATE recordings SET stage_synthesis='{}'",
                initial.as_str()
            ),
        );
        let config = fs::read_to_string(sandbox.config_path()).unwrap();
        fs::write(
            sandbox.config_path(),
            config.split("[synthesis]").next().unwrap(),
        )
        .unwrap();
        json(&invoke(&sandbox, &[]));
        let expected = if initial == StageState::Failed {
            StageState::Disabled
        } else {
            initial
        };
        assert_eq!(
            ledger(&sandbox)
                .by_id(&row.id)
                .unwrap()
                .unwrap()
                .stages
                .synthesis,
            expected
        );
        assert_eq!(
            ledger(&sandbox).accepted_proposal(&row.id).unwrap(),
            proposal
        );
        assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
    }
}
