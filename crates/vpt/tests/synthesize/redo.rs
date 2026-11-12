use super::*;
use vpt_domain::tags;

fn propose(sandbox: &Sandbox, names: &[&str]) {
    let tags: Vec<_> = names
        .iter()
        .map(|name| serde_json::json!({"tag": name, "reason": "private reason"}))
        .collect();
    let proposal = serde_json::json!({
        "schema": "vpt.proposal/1", "by": "private agent", "summary": [], "actions": [],
        "tags": tags, "relations": []
    });
    fs::write(
        sandbox.path().join("captured.proposal"),
        proposal.to_string(),
    )
    .unwrap();
}

#[test]
fn a_redo_drops_automatic_tags_the_new_proposal_omits_and_keeps_hand_set_ones() {
    let (sandbox, record) = note::configured();
    let known = sandbox.path().join("config/vpt/known-tags.txt");
    agent(
        &sandbox,
        &format!("[tags]\nnew_tags = 'write'\nknown_tags_path = {known:?}\n"),
    );
    propose(&sandbox, &["alpha", "beta", "refused"]);
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    for (flag, tag) in [("--tag", "by-hand"), ("--reject-tag", "refused")] {
        note::success(&support::run_with_budget(
            sandbox
                .vpt()
                .args(["confirm", record.id.as_str(), flag, tag, "--json"]),
            Duration::from_secs(5),
        ));
    }
    let vocabulary = fs::read_to_string(&known).unwrap();
    propose(&sandbox, &["alpha"]);
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    let mut names: Vec<_> = input
        .tags
        .unwrap()
        .into_iter()
        .map(|tag| (tag.name, tag.state))
        .collect();
    names.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        names,
        [
            ("alpha".into(), tags::State::Confirmed),
            ("by-hand".into(), tags::State::Confirmed),
            ("refused".into(), tags::State::Rejected)
        ]
    );
    for note in input.notes {
        assert!(!note.text.contains("beta"), "{}", note.text);
        assert!(note.text.contains("by-hand"));
    }
    assert_eq!(fs::read_to_string(&known).unwrap(), vocabulary);
}
