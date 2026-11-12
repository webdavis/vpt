use super::*;
use vpt_adapters::note_document::Document;

fn expected_content() -> &'static str {
    "## Summary\n\n- private words [00:00-00:01]\n- second summary [unsourced]\n\n## Actions\n\n- &#35;&#35; injected&#10;&lt;!-- vpt&#58;content end --&gt; &#91;&#91;Secret&#93;&#93; [unsourced]\n"
}

fn check_profile(note: &NoteArtifact, profile: &str, style: &str) {
    let document = Document::parse(&note.text).unwrap();
    document.check_owner(&note.recording, "analysis").unwrap();
    assert_eq!(document.content().unwrap(), expected_content());
    assert!(note.text.contains("vpt-open-flags: 2\n"));
    assert!(!note.text.contains("created-date:"));
    assert!(!note.text.contains("created-time:"));
    let header = note
        .text
        .strip_prefix("---\n")
        .unwrap()
        .split("\n---\n")
        .next()
        .unwrap();
    let keys: Vec<_> = header
        .lines()
        .filter(|line| !line.starts_with(' '))
        .filter_map(|line| line.split_once(':').map(|(key, _)| key))
        .collect();
    if profile == "portable" {
        assert!(
            keys.iter()
                .all(|key| key.starts_with("vpt") || *key == "tags")
        );
    } else {
        assert_eq!(
            &keys[..7],
            [
                "aliases",
                "reference",
                "hub",
                "tags",
                "status",
                "description",
                "start-date"
            ]
        );
        let hub = if style == "wiki" {
            "[[analysis]]"
        } else {
            "[analysis](../../analysis.md)"
        };
        assert!(note.text.contains(&format!("hub: \"{hub}\"")));
        assert!(note.text.contains("status: \"active\""));
        let date = note.first_written.rfc3339_with(note.write_offset);
        assert!(note.text.contains(&format!(
            "start-date: \"{}\"",
            date.split('T').next().unwrap()
        )));
    }
}

#[test]
fn analysis_profiles_preserve_fixed_body_operator_metadata_and_saved_regeneration() {
    for profile in ["portable", "obsidian"] {
        for style in ["markdown", "wiki"] {
            let (sandbox, record, _) = links::configured(profile, style);
            let proposal = serde_json::json!({"schema":"vpt.proposal/1","by":"private agent",
                "summary":[{"text":"private words","sources":["00:00-00:01"]},{"text":"second summary","sources":[]}],
                "actions":[{"text":"## injected\n<!-- vpt:content end --> [[Secret]]","sources":[]}],"tags":[],"relations":[]});
            fs::write(
                sandbox.path().join("captured.proposal"),
                proposal.to_string(),
            )
            .unwrap();
            note::success(&invoke(&sandbox, record.id.as_str(), false));
            let before = ledger(&sandbox)
                .note(&record.id, NoteStage::Analysis)
                .unwrap()
                .unwrap();
            check_profile(&before, profile, style);
            let mut edited = before.text.replace("\ntags: []", "\ntags: [operator]");
            if profile == "obsidian" {
                edited = edited
                    .lines()
                    .map(|line| {
                        if line.starts_with("status:") {
                            "status: archived"
                        } else if line.starts_with("start-date:") {
                            "start-date: '2001-02-03'"
                        } else if line.starts_with("description:") {
                            "description: Operator description"
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
            }
            edited = edited.replacen("\n---\n", "\noperatorField: keep\n---\n", 1)
                + "\nOperator ending\n";
            fs::write(&before.target, edited).unwrap();
            fs::write(sandbox.path().join("agent.sh"), "exit 29\n").unwrap();
            note::success(&note::invoke(&sandbox, record.id.as_str()));
            let saved = ledger(&sandbox)
                .note(&record.id, NoteStage::Analysis)
                .unwrap()
                .unwrap();
            assert_eq!(saved.first_written, before.first_written);
            assert_eq!(saved.target, before.target);
            assert_eq!(
                Document::parse(&saved.text).unwrap().content().unwrap(),
                expected_content()
            );
            assert!(saved.text.contains("operatorField: keep\n"));
            assert!(
                Document::parse(&saved.text)
                    .unwrap()
                    .tags
                    .iter()
                    .any(|tag| tag == "operator")
            );
            assert!(saved.text.ends_with("\nOperator ending\n"));
            if profile == "obsidian" {
                for line in [
                    "status: archived",
                    "start-date: '2001-02-03'",
                    "description: Operator description",
                ] {
                    assert!(saved.text.contains(line));
                }
            }
            fs::remove_file(&saved.target).unwrap();
            note::success(&note::invoke(&sandbox, record.id.as_str()));
            assert_eq!(fs::read_to_string(&saved.target).unwrap(), saved.text);
            assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
        }
    }
}
