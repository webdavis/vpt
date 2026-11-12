use super::*;
use vpt_domain::{relations, tags};

#[test]
fn analysis_routes_tags_and_related_links_through_the_index_and_caps() {
    for mode in ["hold", "write"] {
        let (sandbox, record) = note::configured();
        let known = sandbox
            .config_path()
            .parent()
            .unwrap()
            .join("known-tags.txt");
        agent(
            &sandbox,
            &format!(
                "[tags]\nnew_tags = '{mode}'\nmax_per_note = 2\nmax_suggested = 1\nknown_tags_path = {known:?}\n"
            ),
        );
        let external = sandbox.path().join("home/.vpt/transcripts/Project.md");
        fs::create_dir_all(external.parent().unwrap()).unwrap();
        let original = "---\ntags: [Established_Tag]\naliases: [Project alias]\n---\n# Project\nExternal text\n";
        fs::write(&external, original).unwrap();
        let proposal = serde_json::json!({
            "schema": "vpt.proposal/1", "by": "private agent", "summary": [], "actions": [],
            "tags": [
                {"tag": "established tag", "reason": "private known reason"},
                {"tag": "New Topic", "reason": "private new reason"},
                {"tag": "new_topic", "reason": "duplicate"},
                {"tag": "Another Topic", "reason": "capped"},
                {"tag": "#bad", "reason": "invalid"}
            ],
            "relations": [{"kind": "related", "target": "Project alias", "reason": "private relation reason"}]
        });
        fs::write(
            sandbox.path().join("captured.proposal"),
            proposal.to_string(),
        )
        .unwrap();
        let output = invoke(&sandbox, record.id.as_str(), false);
        note::success(&output);
        let ledger = ledger(&sandbox);
        let input = ledger.note_inputs(&record.id).unwrap().unwrap();
        let tags = input.tags.unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "Established_Tag");
        assert_eq!(tags[0].state, tags::State::Confirmed);
        assert_eq!(tags[1].name, "new-topic");
        assert_eq!(
            tags[1].state,
            if mode == "hold" {
                tags::State::Suggested
            } else {
                tags::State::Confirmed
            }
        );
        assert!(
            matches!(&tags[1].provenance, tags::Provenance::Proposal { by, reason, .. } if by == "private agent" && reason == "private new reason")
        );
        let relation = input
            .relations
            .iter()
            .find(|link| link.relation.kind == relations::Kind::Related)
            .unwrap();
        assert_eq!(relation.path.as_ref(), Some(&external));
        assert_eq!(relation.relation.state, relations::State::Suggested);
        for note in input.notes {
            assert!(note.text.contains("Established_Tag"));
            assert!(note.text.contains("new-topic"));
            assert!(note.text.contains("private relation reason"));
        }
        assert_eq!(fs::read_to_string(&external).unwrap(), original);
        if mode == "write" {
            assert_eq!(fs::read_to_string(&known).unwrap(), "new-topic\n");
        } else {
            assert!(!known.exists());
        }
        let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
        let rejected: Vec<_> = events
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|row| row["fact"]["tag"] == "tag_rejected")
            .collect();
        assert_eq!(rejected.len(), 2);
        assert!(
            rejected
                .iter()
                .any(|row| row["fact"]["index"] == 4 && row["fact"]["reason"] == "syntax")
        );
        assert!(rejected.iter().any(|row| row["fact"]["index"] == 3
            && row["fact"]["reason"]
                == if mode == "hold" {
                    "suggested_limit"
                } else {
                    "confirmed_limit"
                }));
        for private in [
            "private agent",
            "private new reason",
            "private relation reason",
            "new-topic",
            "Established_Tag",
        ] {
            assert!(!events.contains(private));
            assert!(!stdout(&output).contains(private));
        }
    }
}

#[test]
fn repeated_analysis_preserves_manual_tag_and_relation_decisions() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let external = sandbox.path().join("home/.vpt/transcripts/Project.md");
    fs::create_dir_all(external.parent().unwrap()).unwrap();
    fs::write(&external, "# Project\n").unwrap();
    let proposal = serde_json::json!({
        "schema": "vpt.proposal/1", "by": "private agent", "summary": [], "actions": [],
        "tags": [{"tag": "New Topic", "reason": "private reason"}],
        "relations": [{"kind": "related", "target": "Project", "reason": "private reason"}]
    });
    fs::write(
        sandbox.path().join("captured.proposal"),
        proposal.to_string(),
    )
    .unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    for arguments in [
        ["--reject-tag", "new-topic"],
        ["--relation", "related:Project"],
    ] {
        note::success(&support::run_with_budget(
            sandbox.vpt().args([
                "confirm",
                record.id.as_str(),
                arguments[0],
                arguments[1],
                "--json",
            ]),
            Duration::from_secs(5),
        ));
    }
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    assert_eq!(input.tags.unwrap()[0].state, tags::State::Rejected);
    assert_eq!(
        input
            .relations
            .iter()
            .find(|link| link.relation.kind == relations::Kind::Related)
            .unwrap()
            .relation
            .state,
        relations::State::Confirmed
    );
    for note in input.notes {
        assert!(!note.text.contains("new-topic"));
    }
    assert_eq!(fs::read_to_string(external).unwrap(), "# Project\n");
}
