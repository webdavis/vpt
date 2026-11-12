use super::*;
use vpt_domain::tags::State;

fn configured() -> (Sandbox, Vec<RecordingRecord>, std::path::PathBuf) {
    let (sandbox, rows) = note::configured_with(&[7, 8]);
    let vault = sandbox.path().join("vault");
    let outside = vault.join("Notes/outside.md");
    fs::create_dir_all(outside.parent().unwrap()).unwrap();
    fs::write(&outside, "---\ntags: [billing]\n---\nText\n").unwrap();
    agent(
        &sandbox,
        &format!(
            "[stores]\ntranscripts={:?}\nanalysis={:?}\nbriefs={:?}\n[note]\nprofile='obsidian'\n[note.obsidian]\nvault_root={vault:?}\n[tags]\nknown_tags_path={:?}\n",
            vault.join("Voice/transcripts"),
            vault.join("Voice/analysis"),
            vault.join("Voice/briefs"),
            sandbox.config_path().with_file_name("known-tags.txt"),
        ),
    );
    let config = fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("'{language}'", "'{refused_tags}'");
    fs::write(sandbox.config_path(), config).unwrap();
    for row in &rows {
        note::success(&invoke(&sandbox, row.id.as_str(), false));
    }
    (sandbox, rows, outside)
}

fn vpt(sandbox: &Sandbox, args: &[&str]) {
    note::success(&support::run_with_budget(
        sandbox.vpt().args(args).arg("--json"),
        Duration::from_secs(10),
    ));
}

fn summarize(sandbox: &Sandbox, row: &RecordingRecord, tags: &[&str]) -> Vec<(String, State)> {
    let tags: Vec<_> = tags
        .iter()
        .enumerate()
        .map(|(index, tag)| serde_json::json!({"tag": tag, "reason": format!("reason {index}")}))
        .collect();
    let proposal = serde_json::json!({"schema":"vpt.proposal/1","by":"private agent",
        "summary":[],"actions":[],"tags":tags,"relations":[]});
    fs::write(
        sandbox.path().join("captured.proposal"),
        proposal.to_string(),
    )
    .unwrap();
    note::success(&invoke(sandbox, row.id.as_str(), false));
    ledger(sandbox)
        .tags(&row.id)
        .unwrap()
        .unwrap()
        .into_iter()
        .map(|tag| (tag.name, tag.state))
        .collect()
}

fn renamed_indices(sandbox: &Sandbox) -> Vec<u64> {
    fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|event| event["fact"]["tag"] == "tag_renamed")
        .map(|event| event["fact"]["index"].as_u64().unwrap())
        .collect()
}

#[test]
fn proposed_tags_renamed_away_land_on_their_new_name_through_chains() {
    let (sandbox, rows, outside) = configured();
    vpt(
        &sandbox,
        &["confirm", rows[1].id.as_str(), "--tag", "billing"],
    );
    vpt(&sandbox, &["tag", "rename", "billing", "invoice"]);
    let confirmed = vec![("invoice".to_owned(), State::Confirmed)];
    assert_eq!(
        summarize(&sandbox, &rows[0], &["Billing", "invoice"]),
        confirmed
    );
    assert_eq!(renamed_indices(&sandbox), [0]);
    for note in ledger(&sandbox)
        .note_inputs(&rows[0].id)
        .unwrap()
        .unwrap()
        .notes
    {
        let tags = vpt_adapters::note_document::Document::parse(&note.text)
            .unwrap()
            .tags;
        assert_eq!(tags, ["invoice"]);
    }
    assert!(
        fs::read_to_string(&outside)
            .unwrap()
            .contains("tags: [billing]")
    );
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("billing") && !events.contains("reason 0"));

    vpt(&sandbox, &["tag", "rename", "invoice", "bill"]);
    assert_eq!(
        summarize(&sandbox, &rows[0], &["billing", "invoice", "bill"]),
        [("bill".to_owned(), State::Confirmed)]
    );
    assert_eq!(renamed_indices(&sandbox), [0, 0, 1]);
}

#[test]
fn a_hand_confirmed_old_name_is_kept_and_no_longer_renamed() {
    let (sandbox, rows, _) = configured();
    vpt(
        &sandbox,
        &["confirm", rows[1].id.as_str(), "--tag", "billing"],
    );
    vpt(&sandbox, &["tag", "rename", "billing", "invoice"]);
    vpt(&sandbox, &["tag", "rename", "invoice", "bill"]);
    vpt(
        &sandbox,
        &["confirm", rows[1].id.as_str(), "--tag", "billing"],
    );
    assert_eq!(
        summarize(&sandbox, &rows[0], &["billing"]),
        [("billing".to_owned(), State::Confirmed)]
    );
    assert!(renamed_indices(&sandbox).is_empty());
    let args = fs::read_to_string(sandbox.path().join("captured.args")).unwrap();
    assert_eq!(args.lines().nth(2), Some("invoice -> bill"));
}
