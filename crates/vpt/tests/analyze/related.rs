use super::*;
use vpt_domain::relations::{Kind, Provenance, State};

#[test]
fn related_selection_obeys_default_and_configured_limits_independently_of_tag_mode() {
    for (profile, style) in [("portable", "markdown"), ("obsidian", "wiki")] {
        for (mode, cap) in [("hold", None), ("write", Some(2)), ("hold", Some(0))] {
            let (sandbox, record, vault) = links::configured(profile, style);
            let config = fs::read_to_string(sandbox.config_path()).unwrap();
            let limit = cap.map_or(String::new(), |cap| {
                format!("[relations]\nmax_suggested={cap}\n")
            });
            fs::write(
                sandbox.config_path(),
                format!("{config}\n[tags]\nnew_tags='{mode}'\n{limit}"),
            )
            .unwrap();
            let directory = vault.join("Voice/transcripts");
            fs::create_dir_all(&directory).unwrap();
            for i in 0..8 {
                fs::write(
                    directory.join(format!("Target{i}.md")),
                    "---\naliases: [Ambiguous]\n---\nExternal text\n",
                )
                .unwrap();
            }
            note::success(&invoke(&sandbox, record.id.as_str(), false));
            note::success(&support::run_with_budget(
                sandbox.vpt().args([
                    "confirm",
                    record.id.as_str(),
                    "--relation",
                    "related:Target0",
                    "--json",
                ]),
                Duration::from_secs(5),
            ));
            let mut relations = vec![];
            for target in [record.id.as_str(), "Missing", "Ambiguous", "Target0"] {
                relations.push(
                    serde_json::json!({"kind":"related","target":target,"reason":"unselected"}),
                );
            }
            for i in (1..8).rev() {
                relations.push(serde_json::json!({"kind":"related","target":format!("Target{i}"),"reason":"first & <reason>"}));
                relations.push(serde_json::json!({"kind":"related","target":format!("Target{i}"),"reason":"duplicate reason"}));
            }
            let proposal = serde_json::json!({"schema":"vpt.proposal/1","by":"private <agent>",
                "summary":[],"actions":[],"tags":[],"relations":relations});
            fs::write(
                sandbox.path().join("captured.proposal"),
                proposal.to_string(),
            )
            .unwrap();
            let output = invoke(&sandbox, record.id.as_str(), false);
            note::success(&output);
            let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
            let related: Vec<_> = input
                .relations
                .iter()
                .filter(|link| link.relation.kind == Kind::Related)
                .collect();
            let limit = cap.unwrap_or(5);
            assert_eq!(related.len(), limit + 1);
            assert_eq!(related[0].relation.state, State::Confirmed);
            assert_eq!(related[0].relation.provenance, Provenance::Manual);
            for (link, index) in related[1..].iter().zip(8 - limit..8) {
                assert_eq!(link.path, Some(directory.join(format!("Target{index}.md"))));
                assert_eq!(link.relation.state, State::Suggested);
                assert!(
                    matches!(&link.relation.provenance, Provenance::Proposal { by, reason, input: digest }
                    if by == "private <agent>" && reason == "first & <reason>" && *digest == input.proposal.as_ref().unwrap().input_digest)
                );
            }
            assert_eq!(input.notes.len(), 2);
            for artifact in input.notes {
                assert_eq!(artifact.text.matches("- Related:").count(), limit + 1);
                assert_eq!(
                    artifact
                        .text
                        .matches("(suggested by private &lt;agent&gt;: first &amp; &lt;reason&gt;)")
                        .count(),
                    limit
                );
                assert!(artifact.text.contains("(confirmed manually)"));
                assert!(!artifact.text.contains("duplicate reason"));
                assert!(!artifact.text.contains("unselected"));
                assert_eq!(fs::read_to_string(artifact.target).unwrap(), artifact.text);
            }
            for i in 0..8 {
                assert_eq!(
                    fs::read_to_string(directory.join(format!("Target{i}.md"))).unwrap(),
                    "---\naliases: [Ambiguous]\n---\nExternal text\n"
                );
            }
            let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
            for private in ["first &", "private <agent>", "duplicate reason"] {
                assert!(!events.contains(private));
                assert!(!stdout(&output).contains(private));
            }
        }
    }
}

#[test]
fn agents_cannot_introduce_other_relation_kinds_or_replace_accepted_metadata() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let before = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    for kind in ["supersedes", "continues", "mentions", "unknown"] {
        let proposal = serde_json::json!({"schema":"vpt.proposal/1","by":"private agent",
            "summary":[],"actions":[],"tags":[{"tag":"Private new tag","reason":"private reason"}],
            "relations":[{"kind":kind,"target":"Private target","reason":"private reason"}]});
        fs::write(
            sandbox.path().join("captured.proposal"),
            proposal.to_string(),
        )
        .unwrap();
        let output = invoke(&sandbox, record.id.as_str(), false);
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private"));
        let after = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
        assert_eq!(after.proposal, before.proposal);
        assert_eq!(after.tags, before.tags);
        assert!(after.relations == before.relations);
        assert_eq!(after.notes.len(), before.notes.len());
        for (artifact, previous) in after.notes.iter().zip(&before.notes) {
            let mut expected = previous.clone();
            expected.updated_at = artifact.updated_at;
            assert_eq!(*artifact, expected);
        }
        for artifact in after.notes {
            assert_eq!(fs::read_to_string(artifact.target).unwrap(), artifact.text);
        }
    }
}
