use super::*;

pub(super) fn configured(
    profile: &str,
    style: &str,
) -> (Sandbox, RecordingRecord, std::path::PathBuf) {
    let (sandbox, record) = note::configured();
    let vault = sandbox.path().join("vault");
    fs::create_dir_all(vault.join("Voice")).unwrap();
    agent(
        &sandbox,
        &format!(
            "[stores]\ntranscripts={:?}\nanalysis={:?}\nbriefs={:?}\n[note]\nprofile='{profile}'\nlink_style='{style}'\n[note.obsidian]\nvault_root={vault:?}\n",
            vault.join("Voice/transcripts"),
            vault.join("Voice/analysis"),
            vault.join("Voice/briefs"),
        ),
    );
    (sandbox, record, vault)
}

#[test]
fn obsidian_counterparts_with_the_same_filename_link_to_the_exact_vault_paths() {
    let (sandbox, record, _) = configured("obsidian", "wiki");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    let transcript = input
        .notes
        .iter()
        .find(|note| note.stage == NoteStage::Transcript)
        .unwrap();
    let analysis = input
        .notes
        .iter()
        .find(|note| note.stage == NoteStage::Analysis)
        .unwrap();
    assert_eq!(transcript.target.file_name(), analysis.target.file_name());
    let stem = transcript.target.file_stem().unwrap().to_str().unwrap();
    assert!(
        transcript
            .text
            .contains(&format!("- Analysis: [[Voice/analysis/{stem}|{stem}]]")),
        "{}",
        transcript.text
    );
    assert!(
        analysis.text.contains(&format!(
            "- Transcript: [[Voice/transcripts/{stem}|{stem}]]"
        )),
        "{}",
        analysis.text
    );
    assert!(transcript.text.contains("hub: \"[[transcripts]]\""));
    assert!(analysis.text.contains("hub: \"[[analysis]]\""));
    assert!(
        transcript
            .text
            .contains(&format!("../../../home/.vpt/audio/{}.m4a", record.id))
    );
    for artifact in &input.notes {
        assert_eq!(fs::read_to_string(&artifact.target).unwrap(), artifact.text);
    }
    note::success(&note::invoke(&sandbox, record.id.as_str()));
    let rewritten = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    for (old, new) in input.notes.iter().zip(&rewritten.notes) {
        assert_eq!(old.target, new.target);
        assert_eq!(old.text, new.text);
    }
}

#[test]
fn portable_wiki_counterparts_use_the_common_note_store_root() {
    let (sandbox, record, _) = configured("portable", "wiki");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    for artifact in input.notes {
        let stem = artifact.target.file_stem().unwrap().to_str().unwrap();
        let (kind, folder) = if artifact.stage == NoteStage::Transcript {
            ("Analysis", "analysis")
        } else {
            ("Transcript", "transcripts")
        };
        assert!(
            artifact
                .text
                .contains(&format!("- {kind}: [[{folder}/{stem}|{stem}]]")),
            "{}",
            artifact.text
        );
        assert!(!artifact.text.contains("hub:"));
    }
}

#[test]
fn portable_wiki_root_includes_the_brief_store_and_paths_for_related_notes() {
    let (sandbox, record, vault) = configured("portable", "wiki");
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(
        sandbox.config_path(),
        config.replace("Voice/briefs", "Briefings"),
    )
    .unwrap();
    let external = vault.join("Briefings/Topic.md");
    fs::create_dir_all(external.parent().unwrap()).unwrap();
    fs::write(&external, "External note\n").unwrap();
    let duplicate = vault.join("Voice/transcripts/Topic.md");
    fs::create_dir_all(duplicate.parent().unwrap()).unwrap();
    fs::write(&duplicate, "Other external note\n").unwrap();
    let path = sandbox.path().join("captured.proposal");
    let mut proposal: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    proposal["relations"] =
        serde_json::json!([{"kind":"related","target":external,"reason":"source"}]);
    fs::write(path, proposal.to_string()).unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    for artifact in input.notes {
        let stem = artifact.target.file_stem().unwrap().to_str().unwrap();
        let folder = if artifact.stage == NoteStage::Transcript {
            "analysis"
        } else {
            "transcripts"
        };
        assert!(
            artifact
                .text
                .contains(&format!("[[Voice/{folder}/{stem}|{stem}]]"))
        );
        assert!(artifact.text.contains("[[Briefings/Topic|Topic]]"));
    }
    assert_eq!(fs::read_to_string(external).unwrap(), "External note\n");
    assert_eq!(
        fs::read_to_string(duplicate).unwrap(),
        "Other external note\n"
    );
}

#[test]
fn markdown_counterparts_and_renamed_wiki_targets_preserve_exact_paths_in_both_profiles() {
    for profile in ["portable", "obsidian"] {
        for style in ["markdown", "wiki"] {
            let (sandbox, record, _) = configured(profile, style);
            note::success(&invoke(&sandbox, record.id.as_str(), false));
            let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
            let analysis = input
                .notes
                .iter()
                .find(|note| note.stage == NoteStage::Analysis)
                .unwrap();
            let filename = analysis.target.file_name().unwrap().to_str().unwrap();
            if style == "markdown" {
                assert!(
                    input
                        .notes
                        .iter()
                        .find(|note| note.stage == NoteStage::Transcript)
                        .unwrap()
                        .text
                        .contains(&format!("(../analysis/{filename})"))
                );
            }
            let renamed = analysis.target.with_file_name("Call #1.md");
            fs::rename(&analysis.target, &renamed).unwrap();
            fs::write(&renamed, format!("{}\nOperator ending\n", analysis.text)).unwrap();
            note::success(&note::invoke(&sandbox, record.id.as_str()));
            let rewritten = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
            let transcript = rewritten
                .notes
                .iter()
                .find(|note| note.stage == NoteStage::Transcript)
                .unwrap();
            let analysis = rewritten
                .notes
                .iter()
                .find(|note| note.stage == NoteStage::Analysis)
                .unwrap();
            assert!(
                transcript
                    .text
                    .contains("- Analysis: [Call &#35;1](../analysis/Call%20%231.md)"),
                "{}",
                transcript.text
            );
            assert_eq!(analysis.target, renamed);
            assert!(analysis.text.ends_with("Operator ending\n"));
        }
    }
}
