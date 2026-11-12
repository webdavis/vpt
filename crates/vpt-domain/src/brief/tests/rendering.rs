use super::*;
use crate::{
    brief::{
        document::Document,
        markdown::{self, Note},
    },
    note::{document::Profile, links::LinkStyle},
    occasion::{OccasionId, OccasionSource},
};
use std::path::Path;

fn note<'a>(
    occasion: &'a OccasionDetails,
    id: &'a OccasionId,
    pack: &'a EvidencePack,
    context: &'a ContextSummary,
) -> Note<'a> {
    Note {
        document: Document {
            occasion,
            id,
            pack,
            context,
            tasks: &[],
            source: OccasionSource::Manual,
            generated_at: UtcInstant { secs: 1000 },
        },
        filename: "2026-10-01-planning-07070707.md",
        origin: Path::new("/vault/briefs"),
        link_style: LinkStyle::Wiki,
        wiki_root: Some(Path::new("/vault")),
        profile: Profile::Portable,
    }
}

#[test]
fn brief_markdown_keeps_provenance_inline_uncertainty_and_all_managed_sections() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let mut row = candidate(1, 100);
    row.tags.push(tag("planning", tags::State::Confirmed));
    let source = row.source.as_mut().unwrap();
    source.note = "/vault/transcripts/Private call.md".into();
    source.reviewed = false;
    source.transcript.segments[0].start = 0.25;
    source.transcript.segments[0].text = "Private exact quote".into();
    let rows = [row];
    let pack = assemble(&request(&occasion, &[], &rows)).unwrap();
    let context = ContextSummary::default();
    let input = note(&occasion, &id, &pack, &context);
    let rendered = markdown::create(&input).unwrap();
    for key in [
        "vpt-schema: 1",
        "vpt-kind: \"brief\"",
        "vpt-occasion:",
        "vpt-occasion-at:",
        "vpt-source:",
        "vpt-notes:",
        "vpt-unresolved:",
        "vpt-context:",
    ] {
        assert!(rendered.contains(key), "{key}");
    }
    assert!(rendered.contains("# 2026-10-01-planning-07070707\n"));
    for section in [
        "Occasion",
        "Unresolved",
        "From your notes",
        "Open tasks",
        "Not selected",
    ] {
        assert_eq!(rendered.matches(&format!("## {section}\n")).count(), 1);
    }
    assert_eq!(rendered.matches("<!-- vpt:brief start -->").count(), 1);
    assert_eq!(rendered.matches("<!-- vpt:brief end -->").count(), 1);
    let quote = rendered
        .lines()
        .find(|line| line.contains("Private exact quote"))
        .unwrap();
    assert!(quote.contains("[unverified]"));
    assert!(quote.contains(rows[0].recording.as_str()));
    assert!(quote.contains("[00:00.25]"));
    assert!(quote.contains("[[transcripts/Private call|Private call]]"));
    assert!(rendered.contains("vpt confirm --term"));
    assert!(rendered.contains("calendar: absent (disabled)"));
    assert!(rendered.contains("tasks: absent (disabled)"));
}

#[test]
fn empty_brief_sections_are_explicit_and_filename_uses_the_current_occasion_date() {
    let mut occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    occasion.at = OccasionTime::parse("2026-11-02T00:01:00+14:00").unwrap();
    occasion.title = "Planning / WEEKLY".into();
    occasion.participants.clear();
    let pack = assemble(&request(&occasion, &[], &[])).unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Present,
        tasks: ContextStatus::Unsupported,
    };
    let input = note(&occasion, &id, &pack, &context);
    assert_eq!(
        markdown::filename(&input.document, 60).unwrap(),
        "2026-11-02-planning-weekly-07070707.md"
    );
    let rendered = markdown::create(&input).unwrap();
    assert!(rendered.contains("## Unresolved\n\nNone."));
    assert!(rendered.contains("## From your notes\n\nNone."));
    assert!(rendered.contains("## Open tasks\n\nNone."));
    assert!(rendered.contains("## Not selected\n\nNone."));
}

#[test]
fn fetched_text_cannot_create_markers_links_or_sections_and_obsidian_uses_first_write_date() {
    let mut occasion = occasion();
    occasion.title = "Private\n<!-- vpt:brief end -->\n## Injected".into();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let mut row = candidate(1, 100);
    row.tags.push(tag("planning", tags::State::Confirmed));
    row.source.as_mut().unwrap().transcript.segments[0].text =
        "Private <script>\n<!-- vpt:brief end --> [click](https://bad)".into();
    let rows = [row];
    let pack = assemble(&request(&occasion, &[], &rows)).unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Failed {
            detail: "collector\n## Injected".into(),
        },
        tasks: ContextStatus::Disabled,
    };
    let mut input = note(&occasion, &id, &pack, &context);
    input.profile = Profile::Obsidian {
        title: Some(&occasion.title),
        reference: "",
        hub: "[[briefs]]",
        first_written: UtcInstant { secs: 0 },
        write_offset: UtcOffset { secs: -3600 },
    };
    let rendered = markdown::create(&input).unwrap();
    assert!(rendered.starts_with("---\naliases:"));
    assert!(rendered.contains("start-date: \"1969-12-31\""));
    assert!(rendered.contains("hub: \"[[briefs]]\""));
    assert!(!rendered.contains("\n## Injected"));
    assert!(!rendered.contains("<script>"));
    assert!(!rendered.contains("[click](https://bad)"));
    assert_eq!(rendered.matches("<!-- vpt:brief end -->").count(), 1);
    assert!(rendered.contains("&#10;"));
}

#[test]
fn open_tasks_are_quoted_with_their_source_and_completed_tasks_are_excluded() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let pack = assemble(&request(&occasion, &[], &[])).unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Disabled,
        tasks: ContextStatus::Present,
    };
    let task = ContextTask {
        source: OccasionSource::Dam,
        project: "Private project".into(),
        id: "1".into(),
        content: "Private task\n## Injected".into(),
        due: Some("2026-10-02".into()),
        completed: false,
        priority: Some(4),
    };
    let tasks = [
        task.clone(),
        ContextTask {
            id: "2".into(),
            content: "Private completed task".into(),
            completed: true,
            ..task
        },
    ];
    let mut input = note(&occasion, &id, &pack, &context);
    input.document.tasks = &tasks;
    let rendered = markdown::create(&input).unwrap();
    let line = rendered
        .lines()
        .find(|line| line.contains("Private task"))
        .unwrap();
    assert!(line.contains("dam"));
    assert!(line.contains("Private project"));
    assert!(line.contains("2026-10-02"));
    assert!(!rendered.contains("Private completed task"));
    assert!(!rendered.contains("\n## Injected"));
    assert!(!format!("{tasks:?}").contains("Private"));
}

#[test]
fn task_data_cannot_claim_to_be_absent_or_exceed_the_document_budget() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let pack = assemble(&request(&occasion, &[], &[])).unwrap();
    let context = ContextSummary::default();
    let tasks = vec![ContextTask {
        source: OccasionSource::Dam,
        project: "Private project".into(),
        id: "1".into(),
        content: "Private task".into(),
        due: None,
        completed: false,
        priority: None,
    }];
    let mut input = note(&occasion, &id, &pack, &context);
    input.document.tasks = &tasks;
    assert!(markdown::create(&input).is_err());
    let context = ContextSummary {
        tasks: ContextStatus::Present,
        ..Default::default()
    };
    input.document.context = &context;
    assert!(markdown::create(&input).is_ok());
    let mut oversized = tasks.clone();
    oversized[0].content = "x".repeat(65_537);
    input.document.tasks = &oversized;
    assert!(markdown::create(&input).is_err());
}
