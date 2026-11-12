use super::*;

fn configured() -> (Sandbox, RecordingRecord, std::path::PathBuf) {
    let (sandbox, record, vault) = links::configured("obsidian", "wiki");
    let known = sandbox.config_path().with_file_name("known-tags.txt");
    let config = fs::read_to_string(sandbox.config_path()).unwrap().replace(
        "'{language}'",
        "'Prefer {known_tags}. Never {refused_tags}.'",
    );
    fs::write(
        sandbox.config_path(),
        format!("{config}\n[tags]\nknown_tags_path={known:?}\n"),
    )
    .unwrap();
    (sandbox, record, vault)
}

fn prompt(sandbox: &Sandbox, record: &RecordingRecord) -> String {
    note::success(&invoke(sandbox, record.id.as_str(), false));
    let args = fs::read_to_string(sandbox.path().join("captured.args")).unwrap();
    args.lines().nth(2).unwrap().to_owned()
}

fn vpt(sandbox: &Sandbox, args: &[&str]) {
    note::success(&support::run_with_budget(
        sandbox.vpt().args(args).arg("--json"),
        Duration::from_secs(10),
    ));
}

fn input_digest(sandbox: &Sandbox, record: &RecordingRecord) -> (String, Vec<u8>) {
    let proposal = ledger(sandbox)
        .accepted_proposal(&record.id)
        .unwrap()
        .unwrap();
    (
        proposal.input_digest.hex(),
        fs::read(sandbox.path().join("captured")).unwrap(),
    )
}

#[test]
fn summary_command_receives_known_and_refused_tags_without_changing_its_input() {
    let (sandbox, record, vault) = configured();
    assert_eq!(prompt(&sandbox, &record), "Prefer . Never .");
    let first = input_digest(&sandbox, &record);
    let notes = vault.join("Notes");
    fs::create_dir_all(&notes).unwrap();
    for (name, tags) in [
        ("a", "invoice, garden-club, health"),
        ("b", "invoice, garden-club"),
        ("c", "Invoice"),
        ("d", "billing"),
    ] {
        fs::write(
            notes.join(format!("{name}.md")),
            format!("---\ntags: [{tags}]\n---\nText\n"),
        )
        .unwrap();
    }
    fs::write(
        sandbox.config_path().with_file_name("known-tags.txt"),
        "quiet\nspam\n",
    )
    .unwrap();
    for tag in ["health", "spam"] {
        vpt(
            &sandbox,
            &["confirm", record.id.as_str(), "--reject-tag", tag],
        );
    }
    vpt(
        &sandbox,
        &["confirm", record.id.as_str(), "--tag", "billing"],
    );
    vpt(&sandbox, &["tag", "rename", "billing", "invoice"]);
    let dry = invoke(&sandbox, record.id.as_str(), true);
    note::success(&dry);
    assert!(!stdout(&dry).contains("garden-club"));
    assert_eq!(
        prompt(&sandbox, &record),
        "Prefer invoice (5), garden-club (2), health (1), quiet (0). Never billing -> invoice, spam."
    );
    assert_eq!(input_digest(&sandbox, &record), first);
    vpt(&sandbox, &["confirm", record.id.as_str(), "--tag", "spam"]);
    assert_eq!(
        prompt(&sandbox, &record),
        "Prefer invoice (5), garden-club (2), spam (2), health (1), quiet (0). Never billing -> invoice."
    );
    assert_eq!(input_digest(&sandbox, &record), first);
    vpt(&sandbox, &["tag", "rename", "invoice", "billing"]);
    assert_eq!(
        prompt(&sandbox, &record),
        "Prefer billing (3), garden-club (2), spam (2), health (1), quiet (0). Never invoice -> billing."
    );
}
