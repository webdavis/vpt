use super::*;
mod analysis;
mod content;
mod limits;
mod rewrite;

const ID: &str = "2026-08-24T144736-4f3ab19c02de";
const BODY: &str = "# Filename\n\n<!-- vpt:links start -->\n<!-- vpt:links end -->\n\n<!-- vpt:content start -->\n## Transcript\n\n[00:00] hello\n<!-- vpt:content end -->\n";

fn document(extra: &str) -> String {
    format!("---\nvptSchema: 1\nvptRecording: {ID}\nvptStage: transcript\n{extra}---\n{BODY}")
}

#[test]
fn reads_block_and_flow_metadata_with_unicode_quoted_keys_and_tags() {
    for source in [
        document("aliases: [café, 世界]\n\"tags\": [invoice, 'operator/tag']\n"),
        format!(
            "---\n{{vptSchema: 1, vptRecording: '{ID}', vptStage: transcript, tags: [invoice, 'operator/tag']}}\n---\n{BODY}"
        ),
    ] {
        let parsed = Document::parse(&source).unwrap();
        assert_eq!(parsed.recording.as_str(), ID);
        assert_eq!(parsed.schema, 1);
        assert_eq!(parsed.stage, "transcript");
        assert_eq!(parsed.tags, ["invoice", "operator/tag"]);
        parsed
            .check_owner(&RecordingId::parse(ID).unwrap(), "transcript")
            .unwrap();
        assert_eq!(
            parsed.content().unwrap(),
            "## Transcript\n\n[00:00] hello\n"
        );
    }
}

#[test]
fn future_schema_is_readable_but_never_rewritable() {
    let source = document("").replace("vptSchema: 1", "vptSchema: 2");
    let parsed = Document::parse(&source).unwrap();
    assert_eq!(parsed.schema, 2);
    assert_eq!(
        parsed.check_owner(&RecordingId::parse(ID).unwrap(), "transcript"),
        Err(NoteError::UnsupportedSchema(2))
    );
    assert!(matches!(
        parsed.rewrite(&document(""), &[]),
        Err(NoteError::UnsupportedSchema(2))
    ));
}

#[test]
fn foreign_identity_stage_missing_and_duplicate_keys_are_refused() {
    let valid = document("");
    let parsed = Document::parse(&valid).unwrap();
    let other = RecordingId::parse("2026-08-24T144736-ffffffffffff").unwrap();
    assert_eq!(
        parsed.check_owner(&other, "transcript"),
        Err(NoteError::Identity)
    );
    assert_eq!(
        parsed.check_owner(&parsed.recording, "analysis"),
        Err(NoteError::Stage)
    );
    for source in [
        document("vptSchema: 1\n"),
        document("'vptSchema': 1\n"),
        valid.replace("vptSchema: 1\n", ""),
        valid.replace("vptSchema: 1", "vptSchema: '1'"),
        valid.replace("vptStage: transcript", "vptStage: unknown"),
        valid.replace("vptSchema: 1", "vptSchema: true"),
        valid.replace("vptSchema: 1", "vptSchema: 0"),
        valid.replace(ID, "private invalid identity"),
        document("tags: [one, [nested]]\n"),
        document("status: one\nstatus: two\n"),
    ] {
        assert!(Document::parse(&source).is_err());
    }
}

#[test]
fn aliases_can_supply_metadata_without_unbounded_expansion() {
    let source = format!(
        "---\nversion: &version 1\nidentity: &identity {ID}\nvptSchema: *version\nvptRecording: *identity\nvptStage: transcript\nmytags: &tags [invoice, personal]\ntags: *tags\n---\n{BODY}"
    );
    let parsed = Document::parse(&source).unwrap();
    assert_eq!(parsed.tags, ["invoice", "personal"]);
    for extra in [
        "tags: &cycle [*cycle]\n",
        "tags: *undefined\n",
        "tags: {a: b}\n",
    ] {
        assert!(Document::parse(&document(extra)).is_err());
    }
}

#[test]
fn parse_errors_are_fixed_and_never_echo_scalar_values() {
    let source = document("private: [SECRET_INVALID\n");
    assert!(matches!(
        Document::parse(&source),
        Err(NoteError::Frontmatter)
    ));
    let source = document("").replace("<!-- vpt:content end -->", "");
    assert!(matches!(Document::parse(&source), Err(NoteError::Markers)));
}
