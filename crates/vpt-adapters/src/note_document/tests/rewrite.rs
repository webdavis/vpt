use super::*;

#[test]
fn unicode_spacing_inside_a_plain_owned_value_is_replaced_with_that_value() {
    let source = document("vptLanguage: en-US\u{a0}\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&document("vptLanguage: en-GB\n"), &[])
        .unwrap();
    let parsed = Document::parse(&rewritten).unwrap();
    let field = parsed
        .fields
        .iter()
        .find(|field| field.key == "vptLanguage")
        .unwrap();
    assert_eq!(parsed.tree.string(field.value_node).unwrap(), "en-GB");
}

#[test]
fn removing_a_flow_field_keeps_comments_between_it_and_the_next_field() {
    let source = format!(
        "---\n{{vptSchema: 1, vptObsolete: old\n# keep this comment,\n, vptRecording: '{ID}', vptStage: transcript}}\n---\n{BODY}"
    );
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&document(""), &[])
        .unwrap();
    assert!(rewritten.contains("\n# keep this comment,\n"));
    assert!(!rewritten.contains("vptObsolete"));
    assert!(Document::parse(&rewritten).is_ok());
}

#[test]
fn vpt_values_take_the_new_type_even_when_the_old_value_has_an_explicit_yaml_tag() {
    let source = document("vptOpenFlags: &count !!str 'six'\ncustomCount: *count\n");
    let replacement = document("vptOpenFlags: 3\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &[])
        .unwrap();
    let parsed = Document::parse(&rewritten).unwrap();
    let field = parsed
        .fields
        .iter()
        .find(|field| field.key == "vptOpenFlags")
        .unwrap();
    assert_eq!(parsed.tree.integer(field.value_node).unwrap(), 3);
    assert!(rewritten.contains("customCount: *count"));
}

#[test]
fn encoded_yaml_tag_names_remain_tags_when_their_values_are_rewritten() {
    let source = document("");
    let replacement = document("vptExtra: !<tag:example%3E> payload\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &[])
        .unwrap();
    assert!(rewritten.contains("!<tag:example%3E>"));
    assert!(Document::parse(&rewritten).is_ok());
}

#[test]
fn flow_comment_commas_are_preserved_and_never_mistaken_for_separators() {
    for tail in [" # a comment,\n", ", # trailing separator,\n"] {
        let source = format!(
            "---\n{{vptSchema: 1, vptRecording: '{ID}', vptStage: transcript{tail}}}\n---\n{BODY}"
        );
        let rewritten = Document::parse(&source)
            .unwrap()
            .rewrite(&document("vptOpenFlags: 3\n"), &[])
            .unwrap();
        assert!(rewritten.contains(tail), "{tail:?}: {rewritten:?}");
        let parsed = Document::parse(&rewritten).unwrap();
        let field = parsed
            .fields
            .iter()
            .find(|field| field.key == "vptOpenFlags")
            .unwrap();
        assert_eq!(parsed.tree.integer(field.value_node).unwrap(), 3);
    }
}

#[test]
fn indented_frontmatter_crlf_and_anchored_operator_values_survive() {
    let source = document(
        "custom: &mine [café, 世界]\naliases: *mine\nvptOpenFlags: &count 1\ncountAlias: *count\n",
    )
    .lines()
    .map(|line| {
        if line == "---"
            || line.starts_with('#')
            || line.starts_with("<!--")
            || line.starts_with('[')
            || line.is_empty()
        {
            line.to_owned()
        } else {
            format!("  {line}")
        }
    })
    .collect::<Vec<_>>()
    .join("\r\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&document("vptOpenFlags: 2\nvptLanguage: en-US\n"), &[])
        .unwrap();
    assert!(rewritten.contains("  custom: &mine [café, 世界]\r\n  aliases: *mine\r\n"));
    assert!(rewritten.contains("vptOpenFlags: &count 2"));
    assert!(rewritten.contains("  countAlias: *count\r\n"));
    assert!(Document::parse(&rewritten).is_ok());
}

#[test]
fn rewrite_preserves_operator_fields_comments_and_all_prose_outside_regions() {
    let source = document("status: complete # mine\ndescription: |\n  café\n  second line\nstartDate: 2020-01-01\ncustom: {nested: [one, two]}\n# keep this comment\nvptObsolete: remove-me\ntags: [old-auto, manual]\n")
        .replace("# Filename", "# Renamed heading\nMy preface") + "\nMy ending\n";
    let replacement = document("status: active\ndescription: new\nstartDate: 2026-09-29\nvptOpenFlags: 3\ntags: [new-auto]\n").replace("[00:00] hello", "[00:00] changed");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &["old-auto".into()])
        .unwrap();
    for retained in [
        "status: complete # mine\n",
        "description: |\n  café\n  second line\n",
        "startDate: 2020-01-01\n",
        "custom: {nested: [one, two]}\n",
        "# keep this comment\n",
        "# Renamed heading\nMy preface",
        "\nMy ending\n",
    ] {
        assert!(rewritten.contains(retained), "{retained}");
    }
    assert!(!rewritten.contains("vptObsolete"));
    assert!(!rewritten.contains("old-auto"));
    let parsed = Document::parse(&rewritten).unwrap();
    assert_eq!(parsed.tags, ["manual", "new-auto"]);
    assert_eq!(
        parsed.content().unwrap(),
        "## Transcript\n\n[00:00] changed\n"
    );
}

#[test]
fn flow_frontmatter_preserves_operator_values_and_remains_valid_after_key_removal() {
    let source = format!(
        "---\n{{vptSchema: 1, custom: 'café, 世界', vptObsolete: [old], vptRecording: '{ID}', tags: [manual], vptStage: transcript}}\n---\n{BODY}"
    );
    let replacement = document("vptOpenFlags: 2\ntags: [invoice]\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &[])
        .unwrap();
    assert!(rewritten.contains("custom: 'café, 世界'"));
    assert!(!rewritten.contains("vptObsolete"));
    assert_eq!(
        Document::parse(&rewritten).unwrap().tags,
        ["manual", "invoice"]
    );
}

#[test]
fn rewrite_refuses_a_foreign_or_malformed_replacement_without_changing_the_source() {
    let source = document("status: complete\n");
    let parsed = Document::parse(&source).unwrap();
    for replacement in [
        document("").replace(ID, "2026-08-24T144736-ffffffffffff"),
        document("").replace("transcript\n", "analysis\n"),
        document("").replace("<!-- vpt:links end -->", ""),
    ] {
        assert!(parsed.rewrite(&replacement, &[]).is_err());
    }
    assert_eq!(
        parsed.content().unwrap(),
        "## Transcript\n\n[00:00] hello\n"
    );
}

#[test]
fn unchanged_managed_values_preserve_the_existing_yaml_layout() {
    let source = document("vptEngines:\n  - \"custom:test\"\nvptOpenFlags: 0\ntags: []\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&source, &[])
        .unwrap();
    assert_eq!(rewritten, source);
}

#[test]
fn managed_alias_values_are_still_replaced_when_the_resolved_value_is_unchanged() {
    for value in ["*names", "[*model]"] {
        let source = document(&format!(
            "custom: &model custom:test\nengines: &names [custom:test]\nvptEngines: {value}\n"
        ));
        let replacement = document("vptEngines: [custom:test]\n");
        let rewritten = Document::parse(&source)
            .unwrap()
            .rewrite(&replacement, &[])
            .unwrap();
        assert!(rewritten.contains("vptEngines: [\"custom:test\"]"));
        assert!(rewritten.contains("custom: &model custom:test"));
    }
}
