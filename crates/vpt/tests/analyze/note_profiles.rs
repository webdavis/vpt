use super::*;
use vpt_adapters::note_document::Document;
use vpt_domain::note::managed::Regions;

#[test]
fn audio_inside_the_wiki_root_keeps_its_archive_extension_in_both_notes() {
    for profile in ["portable", "obsidian"] {
        let (sandbox, record) = note::configured();
        let vault = sandbox.path().join("home/.vpt");
        fs::create_dir_all(&vault).unwrap();
        agent(
            &sandbox,
            &format!(
                "[note]\nprofile='{profile}'\nlink_style='wiki'\n[note.obsidian]\nvault_root={vault:?}\n"
            ),
        );
        note::success(&invoke(&sandbox, record.id.as_str(), false));
        let audio = format!("[[audio/{}.m4a|{}.m4a]]", record.id, record.id);
        let notes = ledger(&sandbox)
            .note_inputs(&record.id)
            .unwrap()
            .unwrap()
            .notes;
        assert_eq!(notes.len(), 2);
        for artifact in notes {
            assert!(
                Regions::parse(&artifact.text)
                    .unwrap()
                    .links()
                    .starts_with(&format!("- Audio: {audio}\n"))
            );
            if profile == "obsidian" {
                assert!(artifact.text.contains(&format!("reference: \"{audio}\"\n")));
            }
        }
    }
}

#[test]
fn both_note_profiles_include_audio_and_complete_provenance_with_either_link_style() {
    for profile in ["portable", "obsidian"] {
        for style in ["markdown", "wiki"] {
            let (sandbox, record, _) = links::configured(profile, style);
            note::success(&invoke(&sandbox, record.id.as_str(), false));
            let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
            assert_eq!(input.notes.len(), 2);
            let audio = format!(
                "[{}.m4a](../../../home/.vpt/audio/{}.m4a)",
                record.id, record.id
            );
            for artifact in input.notes {
                let regions = Regions::parse(&artifact.text).unwrap();
                assert!(
                    regions.links().starts_with(&format!("- Audio: {audio}\n")),
                    "{}",
                    artifact.text
                );
                let document = Document::parse(&artifact.text).unwrap();
                document
                    .check_owner(&record.id, artifact.stage.as_str())
                    .unwrap();
                for field in [
                    "vpt-schema: 1\n".into(),
                    format!("vpt-recording: \"{}\"\n", record.id),
                    format!("vpt-stage: \"{}\"\n", artifact.stage.as_str()),
                    format!(
                        "vpt-captured-at: \"{}\"\n",
                        record.captured_at.rfc3339_with(record.captured_offset)
                    ),
                    format!("vpt-duration-secs: {}\n", record.duration_secs),
                    "vpt-language: \"en-US\"\n".into(),
                    "vpt-engines:\n  - \"fixture:test\"\n  - \"comparison:second\"\n".into(),
                    "vpt-open-flags: 1\n".into(),
                    "vpt-diagnostics: 0\n".into(),
                    "vpt-readability: false\n".into(),
                    "vpt-suggested-tags: []\n".into(),
                ] {
                    assert!(
                        artifact.text.contains(&field),
                        "missing {field}: {}",
                        artifact.text
                    );
                }
                assert!(!artifact.text.contains("created-date:"));
                assert!(!artifact.text.contains("created-time:"));
                if profile == "obsidian" {
                    let hub = if artifact.stage == NoteStage::Transcript {
                        "transcripts"
                    } else {
                        "analysis"
                    };
                    let hub = if style == "wiki" {
                        format!("[[{hub}]]")
                    } else {
                        format!("[{hub}](../../{hub}.md)")
                    };
                    let date = artifact.first_written.rfc3339_with(artifact.write_offset);
                    let prefix = format!(
                        "---\naliases:\n  - \"title-7\"\nreference: \"{audio}\"\nhub: \"{hub}\"\ntags: []\nstatus: \"active\"\ndescription: \"Voice memo recorded 2026-08-24\"\nstart-date: \"{}\"\n",
                        date.split('T').next().unwrap()
                    );
                    assert!(artifact.text.starts_with(&prefix), "{}", artifact.text);
                } else {
                    let header = artifact
                        .text
                        .strip_prefix("---\n")
                        .unwrap()
                        .split("\n---\n")
                        .next()
                        .unwrap();
                    assert!(
                        header
                            .lines()
                            .filter(|line| !line.starts_with(' '))
                            .all(|line| line.starts_with("vpt") || line.starts_with("tags:"))
                    );
                }
                assert_eq!(fs::read_to_string(artifact.target).unwrap(), artifact.text);
            }
        }
    }
}
