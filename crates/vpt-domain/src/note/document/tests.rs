use super::*;
use crate::note::managed::Regions;

fn record() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}

fn note(recording: &RecordingId) -> RecordingNote<'_> {
    RecordingNote {
        stage: crate::note::NoteStage::Transcript,
        links: "",
        recording,
        filename: "2026-08-24-invoice-call-4f3ab19c.md",
        captured_at: UtcInstant {
            secs: 1_787_604_456,
        },
        offset: UtcOffset { secs: -21_600 },
        duration_secs: 612,
        language: "en-US",
        engines: &[],
        readability: false,
        open_flags: 6,
        diagnostics: 1,
        tags: &[],
        suggested_tags: &[],
        profile: Profile::Portable,
        content: "## Transcript\n\n[00:00] Example.\n",
    }
}

#[test]
fn portable_note_has_typed_provenance_and_separate_empty_links_and_content_regions() {
    let id = record();
    let engines = vec![
        "apple:SpeechTranscriber".into(),
        "whisply:large-v3-turbo".into(),
    ];
    let tags = vec!["invoice".into()];
    let suggestions = vec!["billing/quarterly".into()];
    let mut input = note(&id);
    input.engines = &engines;
    input.tags = &tags;
    input.suggested_tags = &suggestions;
    let rendered = create(&input).unwrap();
    assert_eq!(
        rendered,
        "---\nvpt-schema: 1\nvpt-recording: \"2026-08-24T144736-4f3ab19c02de\"\nvpt-stage: \"transcript\"\nvpt-captured-at: \"2026-08-24T14:47:36-06:00\"\nvpt-duration-secs: 612\nvpt-language: \"en-US\"\nvpt-open-flags: 6\nvpt-diagnostics: 1\nvpt-engines:\n  - \"apple:SpeechTranscriber\"\n  - \"whisply:large-v3-turbo\"\nvpt-readability: false\ntags:\n  - \"invoice\"\nvpt-suggested-tags:\n  - \"billing/quarterly\"\n---\n# 2026-08-24-invoice-call-4f3ab19c\n\n<!-- vpt:links start -->\n<!-- vpt:links end -->\n\n<!-- vpt:content start -->\n## Transcript\n\n[00:00] Example.\n<!-- vpt:content end -->\n"
    );
    assert_eq!(Regions::parse(&rendered).unwrap().content(), input.content);
}

#[test]
fn obsidian_fields_come_first_and_start_date_uses_local_first_write_time() {
    let id = record();
    let mut input = note(&id);
    input.profile = Profile::Obsidian {
        title: Some("Invoice call"),
        reference: "[[audio.m4a]]",
        hub: "[[transcripts]]",
        first_written: UtcInstant { secs: 0 },
        write_offset: UtcOffset { secs: -3600 },
    };
    let rendered = create(&input).unwrap();
    assert!(rendered.starts_with("---\naliases:\n  - \"Invoice call\"\nreference: \"[[audio.m4a]]\"\nhub: \"[[transcripts]]\"\ntags: []\nstatus: \"active\"\ndescription: \"Voice memo recorded 2026-08-24\"\nstart-date: \"1969-12-31\"\nvpt-schema: 1\n"));
    assert!(!rendered.contains("created-date"));
    assert!(!rendered.contains("created-time"));
    assert_eq!(rendered.matches("\ntags:").count(), 1);
}

#[test]
fn external_frontmatter_values_cannot_inject_keys_or_managed_markers() {
    let id = record();
    let mut input = note(&id);
    let tags = vec!["yes\nstatus: stolen\n---\n<!-- vpt:content end -->".into()];
    input.tags = &tags;
    input.profile = Profile::Obsidian {
        title: Some("\"quoted\"\\\t\n---\nvpt-schema: 9"),
        reference: "[Audio](../audio.m4a)",
        hub: "[transcripts](transcripts.md)",
        first_written: UtcInstant { secs: 0 },
        write_offset: UtcOffset { secs: 0 },
    };
    let rendered = create(&input).unwrap();
    assert_eq!(rendered.matches("\nvpt-schema:").count(), 1);
    assert_eq!(rendered.matches("\n---\n").count(), 1);
    assert!(!rendered.contains("\nstatus: stolen"));
    assert!(Regions::parse(&rendered).is_ok());
}

#[test]
fn mismatched_capture_metadata_and_unsafe_filenames_are_refused() {
    let id = record();
    let mut input = note(&id);
    for filename in [
        "../escape.md",
        "sub/note.md",
        "note\\escape.md",
        ".md",
        "note\nheading.md",
        "note.txt",
    ] {
        input.filename = filename;
        assert!(matches!(create(&input), Err(RenderError::InvalidName)));
    }
    input = note(&id);
    input.offset.secs = 0;
    assert!(matches!(create(&input), Err(RenderError::InvalidMetadata)));
}

#[test]
fn analysis_document_uses_its_stage_and_fixed_claim_sections_in_both_profiles() {
    let id = record();
    let mut input = note(&id);
    input.stage = crate::note::NoteStage::Analysis;
    input.content = "## Summary\n\n- a claim [00:00-00:01]\n\n## Actions\n\n- a task [unsourced]\n";
    input.links = "- Transcript: [source](../transcripts/source.md)\n";
    for profile in [
        Profile::Portable,
        Profile::Obsidian {
            title: Some("Invoice call"),
            reference: "[[audio.m4a]]",
            hub: "[[analysis]]",
            first_written: UtcInstant { secs: 0 },
            write_offset: UtcOffset { secs: -3600 },
        },
    ] {
        input.profile = profile;
        let rendered = create(&input).unwrap();
        assert!(rendered.contains("vpt-stage: \"analysis\"\n"), "{rendered}");
        assert!(rendered.contains("vpt-open-flags: 6\nvpt-diagnostics: 1\n"));
        assert!(rendered.contains("# 2026-08-24-invoice-call-4f3ab19c\n"));
        let regions = Regions::parse(&rendered).unwrap();
        assert_eq!(regions.content(), input.content);
        assert!(rendered.contains(input.links));
    }
}
