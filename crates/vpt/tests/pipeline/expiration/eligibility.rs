use super::*;

#[test]
fn expired_analysis_survives_changed_inputs_and_configuration_disable_enable() {
    let (sandbox, row, artifacts) = prepared();
    let analysis = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Analysis)
        .unwrap();
    age(std::iter::once(&analysis.path));
    json(&invoke(&sandbox, &[]));
    assert!(!analysis.path.exists());
    let proposal = ledger(&sandbox).accepted_proposal(&row.id).unwrap();
    synthesis::append(&sandbox, "\n[readability]\nenabled=true\n");
    let enabled = fs::read_to_string(sandbox.config_path()).unwrap();
    for config in [
        enabled.replace(
            "[synthesis]\ncommand =",
            "[synthesis]\ncommand = []\n# command =",
        ),
        enabled,
    ] {
        fs::write(sandbox.config_path(), config).unwrap();
        let result = json(&invoke(&sandbox, &[]));
        assert_eq!(result["synthesized"], serde_json::json!([]));
        assert_eq!(
            ledger(&sandbox)
                .by_id(&row.id)
                .unwrap()
                .unwrap()
                .stages
                .synthesis,
            StageState::Expired
        );
        assert!(!analysis.path.exists());
        assert_eq!(
            ledger(&sandbox).accepted_proposal(&row.id).unwrap(),
            proposal
        );
    }
    assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
    assert_eq!(synthesis::calls(&sandbox, "calls"), 1);
}
