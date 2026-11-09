use super::*;

#[test]
fn updating_flag_count_preserves_other_yaml_values_comments_and_source_lines() {
    for source in [
        document("vptOpenFlags: 9 # keep comment\ncustom: 'operator'\n"),
        format!(
            "---\n{{vptSchema: 1, vptRecording: '{ID}', vptStage: transcript, vptOpenFlags: 9, custom: 'operator'}}\n---\n{BODY}"
        ),
    ] {
        let source = source.replace('\n', "\r\n");
        let parsed = Document::parse(&source).unwrap();
        assert_eq!(
            parsed.with_open_flags(2).unwrap(),
            source.replace("vptOpenFlags: 9", "vptOpenFlags: 2")
        );
        let first = source
            .lines()
            .position(|line| line == "## Transcript")
            .unwrap()
            + 1;
        assert_eq!(parsed.content_start_line(), first);
    }
    let source = document("");
    assert!(
        Document::parse(&source)
            .unwrap()
            .with_open_flags(0)
            .unwrap()
            .contains("vptOpenFlags: 0\n")
    );
}

#[test]
fn annotation_update_preserves_every_byte_outside_the_content_region() {
    for stage in ["transcript", "analysis"] {
        let source = format!(
            "{}\nOperator ending\n",
            document("# Private comment\ntags: [operator/tag]\nstatus: 'reviewed'\n")
        )
        .replace("vptStage: transcript", &format!("vptStage: {stage}"))
        .replace(
            "<!-- vpt:links end -->",
            "- [Link](private.md)\n<!-- vpt:links end -->",
        )
        .replace("# Filename", "# Filename\n\nOperator preface")
        .replace('\n', "\r\n");
        let parsed = Document::parse(&source).unwrap();
        let next = parsed
            .content()
            .unwrap()
            .replace("hello", "hello [unsourced]");
        let rendered = parsed.replace_content(&next).unwrap();
        assert_eq!(rendered, source.replace("hello", "hello [unsourced]"));
        assert_eq!(
            Document::parse(&rendered)
                .unwrap()
                .replace_content(&next)
                .unwrap(),
            rendered
        );
    }
}

#[test]
fn content_only_updates_refuse_future_schema_and_marker_injection() {
    let future = document("").replace("vptSchema: 1", "vptSchema: 2");
    assert_eq!(
        Document::parse(&future)
            .unwrap()
            .replace_content("changed\n"),
        Err(NoteError::UnsupportedSchema(2))
    );
    let source = document("");
    assert_eq!(
        Document::parse(&source)
            .unwrap()
            .replace_content("<!-- vpt:content end -->\n"),
        Err(NoteError::Markers)
    );
}
