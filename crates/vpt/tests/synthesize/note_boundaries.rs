use super::*;
use vpt_adapters::note_document::Document;
use vpt_domain::note::managed::{CONTENT_END, CONTENT_START, LINKS_START, Regions};

#[test]
fn rewriting_both_notes_restores_managed_regions_and_preserves_operator_bytes() {
    for (profile, style) in [("portable", "wiki"), ("obsidian", "markdown")] {
        let (sandbox, record, _) = links::configured(profile, style);
        note::success(&invoke(&sandbox, record.id.as_str(), false));
        let before = ledger(&sandbox)
            .note_inputs(&record.id)
            .unwrap()
            .unwrap()
            .notes;
        for artifact in &before {
            let edited = Regions::parse(&artifact.text)
                .unwrap()
                .rewrite("Old generated links\n", "Old generated content\n")
                .unwrap()
                .replace(
                    LINKS_START,
                    &format!("Operator preface: café\n\n{LINKS_START}"),
                )
                .replace(
                    CONTENT_START,
                    &format!("Operator middle: 世界\n\n{CONTENT_START}"),
                )
                .replace("tags: []", "tags: [operator]")
                .replace("vptOpenFlags: 1", "vptOpenFlags: 77")
                .replace("status: \"active\"", "status: archived # keep comment")
                .replace(
                    "description: \"Voice memo recorded 2026-08-24\"",
                    "description: |\n  Operator description\n  second line",
                )
                .replacen("startDate: ", "startDate: &first ", 1)
                .replacen("\n---\n", "\ncustom: {nested: [one, two]} # mine\n---\n", 1)
                + "\nOperator ending\n";
            fs::write(&artifact.target, edited).unwrap();
        }
        fs::write(sandbox.path().join("agent.sh"), "exit 29\n").unwrap();
        note::success(&note::invoke(&sandbox, record.id.as_str()));
        let after = ledger(&sandbox)
            .note_inputs(&record.id)
            .unwrap()
            .unwrap()
            .notes;
        for (previous, artifact) in before.iter().zip(after) {
            let old = Regions::parse(&previous.text).unwrap();
            let new = Regions::parse(&artifact.text).unwrap();
            assert_eq!(new.links(), old.links());
            assert_eq!(new.content(), old.content());
            assert_eq!(artifact.first_written, previous.first_written);
            assert_eq!(artifact.write_offset, previous.write_offset);
            assert_eq!(artifact.content_digest, previous.content_digest);
            assert_eq!(Document::parse(&artifact.text).unwrap().tags, ["operator"]);
            for preserved in [
                "Operator preface: café\n\n",
                "Operator middle: 世界\n\n",
                "custom: {nested: [one, two]} # mine\n",
            ] {
                assert!(artifact.text.contains(preserved));
            }
            assert!(artifact.text.ends_with("\nOperator ending\n"));
            assert!(artifact.text.contains("vptOpenFlags: 1\n"));
            if profile == "obsidian" {
                assert!(artifact.text.contains("status: archived # keep comment\n"));
                assert!(
                    artifact
                        .text
                        .contains("description: |\n  Operator description\n  second line\n")
                );
                assert!(artifact.text.contains("startDate: &first "));
            }
            assert_eq!(fs::read_to_string(artifact.target).unwrap(), artifact.text);
        }
    }
}

#[test]
fn malformed_or_foreign_note_refuses_the_whole_two_note_rewrite() {
    for (profile, style) in [("portable", "markdown"), ("obsidian", "wiki")] {
        let (sandbox, record, _) = links::configured(profile, style);
        note::success(&invoke(&sandbox, record.id.as_str(), false));
        let before = ledger(&sandbox)
            .note_inputs(&record.id)
            .unwrap()
            .unwrap()
            .notes;
        for damaged in &before {
            let other_stage = if damaged.stage == NoteStage::Transcript {
                "analysis"
            } else {
                "transcript"
            };
            for (edited, rule) in [
                (damaged.text.replace(CONTENT_END, ""), "markers"),
                (
                    damaged
                        .text
                        .replace(LINKS_START, &format!("{LINKS_START}\n{LINKS_START}")),
                    "markers",
                ),
                (
                    damaged.text.replace(
                        &format!("vptStage: \"{}\"", damaged.stage.as_str()),
                        &format!("vptStage: \"{other_stage}\""),
                    ),
                    "foreign_note",
                ),
                (
                    damaged
                        .text
                        .replace(record.id.as_str(), "2026-08-24T144743-ffffffffffff"),
                    "foreign_note",
                ),
                (
                    damaged.text.replace("vptSchema: 1", "vptSchema: 2"),
                    "schema_ahead",
                ),
            ] {
                fs::write(&damaged.target, &edited).unwrap();
                let output = note::invoke(&sandbox, record.id.as_str());
                assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
                assert!(output.stdout.is_empty());
                let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
                assert_eq!(error["error"]["rule"], rule);
                assert!(stderr(&output).contains(damaged.target.to_str().unwrap()));
                assert_eq!(
                    ledger(&sandbox)
                        .note_inputs(&record.id)
                        .unwrap()
                        .unwrap()
                        .notes,
                    before
                );
                assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
                for artifact in &before {
                    assert_eq!(
                        fs::read_to_string(&artifact.target).unwrap(),
                        if artifact.target == damaged.target {
                            &edited
                        } else {
                            &artifact.text
                        }
                        .as_str()
                    );
                }
                fs::write(&damaged.target, &damaged.text).unwrap();
            }
        }
    }
}
