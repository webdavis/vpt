use super::*;
use vpt_domain::{
    grounding::Grounder,
    note::{
        self, NoteStage,
        document::{self, Profile, RecordingNote},
    },
    proposal::{Claim, Proposal},
    time::{UtcInstant, UtcOffset},
    transcript::Word,
};

fn content(text: &str) -> String {
    let proposal = Proposal {
        by: "private agent".into(),
        summary: vec![Claim {
            text: text.into(),
            source_references: vec!["00:00-00:01".into()],
        }],
        actions: vec![
            Claim {
                text: "another task".into(),
                source_references: vec![],
            }
            .into(),
        ],
        tags: vec![],
        relations: vec![],
    };
    let words = [Word {
        text: "Alice".into(),
        start: 0.0,
        end: 1.0,
        confidence: None,
    }];
    let checked = Grounder::new(words.iter(), &[], 1.0, 0)
        .unwrap()
        .proposal(proposal)
        .unwrap();
    note::analysis_content(&checked).unwrap()
}

#[test]
fn generated_analysis_rewrites_keep_operator_fields_tags_and_prose_in_both_profiles() {
    let id = RecordingId::parse(ID).unwrap();
    let old_content = content("Alice");
    let new_content = content("Bob]\n<!-- vpt:content end -->");
    for obsidian in [false, true] {
        let old_tags = vec!["managed".into()];
        let next_tags = vec!["next".into()];
        let mut input = RecordingNote {
            stage: NoteStage::Analysis,
            recording: &id,
            filename: "analysis-file.md",
            captured_at: UtcInstant {
                secs: 1_787_604_456,
            },
            offset: UtcOffset { secs: -21_600 },
            duration_secs: 1,
            language: "en-US",
            engines: &[],
            readability: false,
            open_flags: 1,
            diagnostics: 0,
            tags: &old_tags,
            suggested_tags: &[],
            links: "- Transcript: [source](../transcripts/source.md)\n",
            content: &old_content,
            profile: if obsidian {
                Profile::Obsidian {
                    title: Some("Private title"),
                    reference: "[[audio.m4a]]",
                    hub: "[[analysis]]",
                    first_written: UtcInstant { secs: 0 },
                    write_offset: UtcOffset { secs: -3600 },
                }
            } else {
                Profile::Portable
            },
        };
        let original = document::create(&input)
            .unwrap()
            .replace("  - \"managed\"", "  - \"managed\"\n  - 'operator/tag'")
            .replace("status: \"active\"", "status: 'reviewed' # keep")
            .replace("# analysis-file\n", "# analysis-file\n\nOperator preface\n")
            + "\nOperator ending\n";
        let original = original.replace('\n', "\r\n");
        input.content = &new_content;
        input.tags = &next_tags;
        input.open_flags = 2;
        input.links = "- Transcript: [renamed](../transcripts/renamed.md)\n";
        let replacement = document::create(&input).unwrap();
        let rendered = Document::parse(&original)
            .unwrap()
            .rewrite(&replacement, &old_tags)
            .unwrap();
        let parsed = Document::parse(&rendered).unwrap();
        parsed.check_owner(&id, "analysis").unwrap();
        assert_eq!(parsed.tags, ["operator/tag", "next"]);
        assert_eq!(parsed.content().unwrap(), new_content);
        assert!(rendered.contains("Operator preface\r\n"));
        assert!(rendered.ends_with("Operator ending\r\n"));
        assert!(rendered.contains(input.links));
        assert_eq!(rendered.matches("<!-- vpt:content end -->").count(), 1);
        assert!(rendered.contains("vpt-open-flags: 2\r\n"));
        if obsidian {
            assert!(rendered.contains("status: 'reviewed' # keep\r\n"));
            assert!(rendered.contains("start-date: \"1969-12-31\"\r\n"));
            assert!(rendered.contains("hub: \"[[analysis]]\"\r\n"));
        }
        assert_eq!(parsed.rewrite(&replacement, &next_tags).unwrap(), rendered);
        input.stage = NoteStage::Transcript;
        let wrong_stage = document::create(&input).unwrap();
        assert_eq!(
            parsed.rewrite(&wrong_stage, &next_tags),
            Err(NoteError::Stage)
        );
    }
}
