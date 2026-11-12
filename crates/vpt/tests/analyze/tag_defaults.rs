use super::*;
use vpt_adapters::note_document::Document;
use vpt_domain::tags::{Provenance, State};

#[test]
fn default_tag_limits_keep_proposal_order_and_operator_tags_in_both_profiles() {
    for profile in ["portable", "obsidian"] {
        for mode in ["hold", "write"] {
            let (sandbox, record, vault) = links::configured(profile, "wiki");
            let known = sandbox.config_path().with_file_name("known-tags.txt");
            let config = fs::read_to_string(sandbox.config_path()).unwrap();
            let switch = if mode == "write" {
                "new_tags='write'\n"
            } else {
                ""
            };
            fs::write(
                sandbox.config_path(),
                format!("{config}\n[tags]\n{switch}known_tags_path={known:?}\n"),
            )
            .unwrap();
            note::success(&invoke(&sandbox, record.id.as_str(), false));
            for artifact in ledger(&sandbox)
                .note_inputs(&record.id)
                .unwrap()
                .unwrap()
                .notes
            {
                let operator = format!("operator-{}", artifact.stage.as_str());
                let edited = artifact
                    .text
                    .replace("\ntags: []", &format!("\ntags: [{operator}]"))
                    + "\nOperator ending\n";
                fs::write(&artifact.target, edited).unwrap();
            }
            let external = vault.join("Voice/transcripts/Vocabulary.md");
            let names: Vec<_> = (0..7).map(|i| format!("Known_{i}")).collect();
            let original = format!("---\ntags: [{}]\n---\nExternal text\n", names.join(", "));
            fs::write(&external, &original).unwrap();
            let mut tags = Vec::new();
            for i in 0..12 {
                if i < 7 {
                    tags.push(serde_json::json!({"tag":format!("known {i}"),"reason":format!("known reason {i}")}));
                }
                tags.push(serde_json::json!({"tag":format!("New {i}"),"reason":format!("first reason {i}")}));
                tags.push(
                    serde_json::json!({"tag":format!("NEW_{i}"),"reason":"duplicate reason"}),
                );
            }
            let proposal = serde_json::json!({"schema":"vpt.proposal/1","by":"private agent",
                "summary":[],"actions":[],"tags":tags,
                "relations":[{"kind":"related","target":"Vocabulary","reason":"independent relation"}]});
            fs::write(
                sandbox.path().join("captured.proposal"),
                proposal.to_string(),
            )
            .unwrap();
            let output = invoke(&sandbox, record.id.as_str(), false);
            note::success(&output);
            let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
            let tags = input.tags.unwrap();
            let confirmed: Vec<_> = tags
                .iter()
                .filter(|tag| tag.state == State::Confirmed)
                .map(|tag| tag.name.as_str())
                .collect();
            let suggested: Vec<_> = tags
                .iter()
                .filter(|tag| tag.state == State::Suggested)
                .map(|tag| tag.name.clone())
                .collect();
            assert_eq!(
                confirmed,
                if mode == "hold" {
                    vec!["Known_0", "Known_1", "Known_2", "Known_3", "Known_4"]
                } else {
                    vec!["Known_0", "new-0", "Known_1", "new-1", "Known_2"]
                }
            );
            assert_eq!(
                suggested,
                if mode == "hold" {
                    (0..10).map(|i| format!("new-{i}")).collect::<Vec<_>>()
                } else {
                    vec![]
                }
            );
            for tag in tags.iter().filter(|tag| tag.name.starts_with("new-")) {
                let index = tag.name.strip_prefix("new-").unwrap();
                assert!(
                    matches!(&tag.provenance, Provenance::Proposal { by, reason, .. }
                    if by == "private agent" && reason == &format!("first reason {index}"))
                );
            }
            assert_eq!(input.notes.len(), 2);
            for artifact in input.notes {
                let document = Document::parse(&artifact.text).unwrap();
                let mut expected: Vec<_> = confirmed.iter().map(|name| name.to_string()).collect();
                expected.push(format!("operator-{}", artifact.stage.as_str()));
                expected.sort();
                let mut actual = document.tags;
                actual.sort();
                assert_eq!(actual, expected);
                let suggested_value = artifact
                    .text
                    .lines()
                    .find_map(|line| line.strip_prefix("vpt-suggested-tags: "))
                    .unwrap();
                let actual: Vec<String> = serde_json::from_str(suggested_value).unwrap();
                assert_eq!(actual, suggested);
                assert!(artifact.text.contains("independent relation"));
                assert!(artifact.text.ends_with("\nOperator ending\n"));
                assert_eq!(fs::read_to_string(artifact.target).unwrap(), artifact.text);
            }
            if mode == "hold" {
                assert!(!known.exists());
            } else {
                assert_eq!(fs::read_to_string(known).unwrap(), "new-0\nnew-1\n");
            }
            assert_eq!(fs::read_to_string(external).unwrap(), original);
            let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
            let rejected: Vec<serde_json::Value> = events
                .lines()
                .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                .filter(|event| event["fact"]["tag"] == "tag_rejected")
                .map(|event| event["fact"].clone())
                .collect();
            assert_eq!(rejected.len(), if mode == "hold" { 4 } else { 14 });
            if mode == "hold" {
                for (index, reason) in [
                    (15, "confirmed_limit"),
                    (18, "confirmed_limit"),
                    (27, "suggested_limit"),
                    (29, "suggested_limit"),
                ] {
                    assert!(
                        rejected
                            .iter()
                            .any(|fact| fact["index"] == index && fact["reason"] == reason)
                    );
                }
            }
            for private in [
                "first reason",
                "duplicate reason",
                "new-0",
                "Known_0",
                "independent relation",
            ] {
                assert!(!events.contains(private));
                assert!(!stdout(&output).contains(private));
            }
        }
    }
}
