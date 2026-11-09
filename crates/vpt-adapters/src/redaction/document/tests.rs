use super::*;
use vpt_domain::redaction::Options;
fn id() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}
fn owned() -> String {
    format!(
        "---\nvptSchema: 1\nvptRecording: {}\nvptStage: transcript\ntitle: PRIVATE\ntags: [PRIVATE]\n---\nOperator PRIVATE\n<!-- vpt:links start -->\n[PRIVATE](secret)\n<!-- vpt:links end -->\n<!-- vpt:content start -->\nAlice met Atlas.\n<!-- vpt:content end -->\nOperator tail PRIVATE\n",
        id()
    )
}
#[test]
fn owned_content_excludes_frontmatter_operator_prose_and_links_with_exact_offsets() {
    let source = owned();
    let (content, offset) = owned_content(&source, &id(), NoteStage::Transcript).unwrap();
    assert_eq!(content, "Alice met Atlas.\n");
    assert_eq!(&source[offset..offset + content.len()], content);
    assert!(owned_content(&source, &id(), NoteStage::Analysis).is_err());
    let foreign = RecordingId::parse("2026-08-24T144736-ffffffffffff").unwrap();
    assert!(owned_content(&source, &foreign, NoteStage::Transcript).is_err());
}
#[test]
fn release_header_is_minimal_and_source_line_mapping_accounts_for_generated_frontmatter() {
    let source = owned();
    let (content, _) = owned_content(&source, &id(), NoteStage::Transcript).unwrap();
    let mut redactor = Redactor::new(&["Alice"], &["Atlas"], Options::default()).unwrap();
    let content = redact_content(content, &mut redactor, &ContentOptions::default()).unwrap();
    let header = Header {
        title: Some("Public extract".into()),
        date: Some("2026-08-24".into()),
        ..Default::default()
    };
    let released = assemble(content, &header, &redactor).unwrap();
    assert!(released.text.starts_with("---\ntitle: \"Public extract\"\ndate: \"2026-08-24\"\nsource: \"redacted extract, not a verbatim record\"\n---\n\n"));
    assert_eq!(released.lines[0].line, 7);
    assert!(!released.text.contains("PRIVATE"));
    assert!(!released.text.contains("vpt"));
    assert!(!released.text.contains("tags:"));
}
#[test]
fn metadata_residue_refuses_private_values_and_empty_source_omits_frontmatter() {
    for (title, source) in [
        (Some("Alice".into()), "public"),
        (None, "private@example.org"),
    ] {
        let mut redactor = Redactor::new(&["Alice"], &[], Options::default()).unwrap();
        let content = redact_content(
            "private@example.org",
            &mut redactor,
            &ContentOptions::default(),
        )
        .unwrap();
        let error = assemble(
            content,
            &Header {
                title,
                date: None,
                source: source.into(),
            },
            &redactor,
        )
        .unwrap_err();
        assert!(matches!(error, Error::Residue { .. }));
        assert!(!format!("{error:?}").contains("private"));
    }
    let mut redactor = Redactor::new(&[], &[], Options::default()).unwrap();
    let content = redact_content("Public body", &mut redactor, &ContentOptions::default()).unwrap();
    let result = assemble(
        content,
        &Header {
            source: String::new(),
            ..Default::default()
        },
        &redactor,
    )
    .unwrap();
    assert_eq!(result.text, "Public body\n");
}
