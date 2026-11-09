use super::*;
use crate::{
    ledger::contract::proposals::fixture,
    note_document::{Document, NoteFormatter},
};
use vpt_application::{
    note::{WriteOptions, prepare},
    ports::*,
};
use vpt_domain::{
    review::{FlagContent, Resolution, ReviewFlag},
    time::{UtcInstant, UtcOffset},
};

#[test]
fn first_analysis_prepares_both_notes_and_findings_without_publishing() {
    for profile in ["portable", "obsidian"] {
        for style in ["markdown", "wiki"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            let mut settings = crate::config::from_table(
                &crate::config::load_text(&format!(
                    "config_version = 1\n[note]\nprofile = '{profile}'\nlink_style = '{style}'\n[note.obsidian]\nvault_root = '{}'", root.display()
                ))
                .unwrap(),
                root,
            )
            .unwrap();
            settings.stores.transcripts = root.join("transcripts");
            settings.stores.analysis = root.join("analysis");
            for directory in [&settings.stores.transcripts, &settings.stores.analysis] {
                std::fs::create_dir(directory).unwrap();
            }
            let stores = FilesystemStores::open(&[
                settings.stores.transcripts.clone(),
                settings.stores.analysis.clone(),
            ])
            .unwrap();
            let ledger = crate::MemoryLedger::new();
            fixture::seed(&ledger);
            let proposal = fixture::accepted(&ledger);
            let mut input = ledger.note_inputs(&proposal.recording).unwrap().unwrap();
            let transcript_path = settings.stores.transcripts.join("source.md");
            input.notes[0].target = transcript_path.clone();
            input.notes[0].text = input.notes[0].text.replace(
                "vptStage: transcript\n",
                "vptStage: transcript\nvptOpenFlags: 1\n",
            );
            input.notes[0].text.push_str("\nOperator ending\n");
            input.notes[0].digest = stores.digest_bytes(input.notes[0].text.as_bytes());
            std::fs::write(&transcript_path, &input.notes[0].text).unwrap();
            let original = input.notes[0].text.clone();
            let content = vpt_domain::note::analysis_content(&proposal.proposal).unwrap();
            let checked = crate::verification::check(
                &content,
                Some(NoteStage::Analysis),
                input.review.accepted.as_ref().unwrap(),
                &input.review.flags,
                0,
            )
            .unwrap();
            input
                .review
                .flags
                .extend(checked.findings.into_iter().map(|finding| ReviewFlag {
                    identifier: crate::verification_flag_id(&proposal.recording, &finding),
                    content: FlagContent::Verification(finding),
                    resolution: Resolution::Open,
                }));
            input.proposal = Some(proposal);
            let delta = prepare(
                &input,
                &settings,
                WriteOptions {
                    roots: &settings.stores,
                    regenerate_expired: false,
                    analysis_content: Some(&checked.content),
                },
                UtcInstant { secs: 70 },
                UtcOffset { secs: -3600 },
                &stores,
                &NoteFormatter,
            )
            .unwrap();
            assert_eq!(delta.notes.len(), 2, "initial synthesis needs both notes");
            assert_eq!(delta.commit.publications.len(), 2);
            let transcript = &delta.notes[0];
            let analysis = &delta.notes[1];
            assert_eq!(analysis.artifact.stage, NoteStage::Analysis);
            assert_eq!(analysis.expected_saved, None);
            assert_eq!(
                analysis.artifact.target.parent(),
                Some(settings.stores.analysis.as_path())
            );
            assert_eq!(analysis.artifact.first_written.secs, 70);
            assert_eq!(analysis.artifact.write_offset.secs, -3600);
            let parsed = Document::parse(&analysis.artifact.text).unwrap();
            parsed.check_owner(&input.record.id, "analysis").unwrap();
            assert_eq!(parsed.content().unwrap(), checked.content);
            assert!(analysis.artifact.text.contains("- Transcript:"));
            assert!(analysis.artifact.text.contains("source"));
            assert!(transcript.artifact.text.contains("- Analysis:"));
            assert!(transcript.artifact.text.ends_with("\nOperator ending\n"));
            for note in &delta.notes {
                assert!(note.artifact.text.contains("vptOpenFlags: 3"));
                assert_eq!(
                    note.artifact.digest,
                    stores.digest_bytes(note.artifact.text.as_bytes())
                );
            }
            assert_eq!(std::fs::read_to_string(transcript_path).unwrap(), original);
            assert_eq!(
                std::fs::read_dir(&settings.stores.analysis)
                    .unwrap()
                    .count(),
                0
            );
            if profile == "obsidian" {
                assert!(analysis.artifact.text.contains("startDate: \"1969-12-31\""));
            }
        }
    }
}
