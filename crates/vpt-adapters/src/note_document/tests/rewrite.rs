use super::*;

#[test]
fn unicode_spacing_inside_a_plain_owned_value_is_replaced_with_that_value() {
    let source = document("vpt-language: en-US\u{a0}\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&document("vpt-language: en-GB\n"), &[])
        .unwrap();
    let parsed = Document::parse(&rewritten).unwrap();
    let field = parsed
        .fields
        .iter()
        .find(|field| field.key == "vpt-language")
        .unwrap();
    assert_eq!(parsed.tree.string(field.value_node).unwrap(), "en-GB");
}

#[test]
fn removing_a_flow_field_keeps_comments_between_it_and_the_next_field() {
    let source = format!(
        "---\n{{vpt-schema: 1, vpt-obsolete: old\n# keep this comment,\n, vpt-recording: '{ID}', vpt-stage: transcript}}\n---\n{BODY}"
    );
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&document(""), &[])
        .unwrap();
    assert!(rewritten.contains("\n# keep this comment,\n"));
    assert!(!rewritten.contains("vpt-obsolete"));
    assert!(Document::parse(&rewritten).is_ok());
}

#[test]
fn vpt_values_take_the_new_type_even_when_the_old_value_has_an_explicit_yaml_tag() {
    let source = document("vpt-open-flags: &count !!str 'six'\ncustomCount: *count\n");
    let replacement = document("vpt-open-flags: 3\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &[])
        .unwrap();
    let parsed = Document::parse(&rewritten).unwrap();
    let field = parsed
        .fields
        .iter()
        .find(|field| field.key == "vpt-open-flags")
        .unwrap();
    assert_eq!(parsed.tree.integer(field.value_node).unwrap(), 3);
    assert!(rewritten.contains("customCount: *count"));
}

#[test]
fn encoded_yaml_tag_names_remain_tags_when_their_values_are_rewritten() {
    let source = document("");
    let replacement = document("vpt-extra: !<tag:example%3E> payload\n");
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
            "---\n{{vpt-schema: 1, vpt-recording: '{ID}', vpt-stage: transcript{tail}}}\n---\n{BODY}"
        );
        let rewritten = Document::parse(&source)
            .unwrap()
            .rewrite(&document("vpt-open-flags: 3\n"), &[])
            .unwrap();
        assert!(rewritten.contains(tail), "{tail:?}: {rewritten:?}");
        let parsed = Document::parse(&rewritten).unwrap();
        let field = parsed
            .fields
            .iter()
            .find(|field| field.key == "vpt-open-flags")
            .unwrap();
        assert_eq!(parsed.tree.integer(field.value_node).unwrap(), 3);
    }
}

#[test]
fn indented_frontmatter_crlf_and_anchored_operator_values_survive() {
    let source = document(
        "custom: &mine [café, 世界]\naliases: *mine\nvpt-open-flags: &count 1\ncountAlias: *count\n",
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
        .rewrite(&document("vpt-open-flags: 2\nvpt-language: en-US\n"), &[])
        .unwrap();
    assert!(rewritten.contains("  custom: &mine [café, 世界]\r\n  aliases: *mine\r\n"));
    assert!(rewritten.contains("vpt-open-flags: &count 2"));
    assert!(rewritten.contains("  countAlias: *count\r\n"));
    assert!(Document::parse(&rewritten).is_ok());
}

#[test]
fn rewrite_preserves_operator_fields_comments_and_all_prose_outside_regions() {
    let source = document("status: complete # mine\ndescription: |\n  café\n  second line\nstart-date: 2020-01-01\ncustom: {nested: [one, two]}\n# keep this comment\nvpt-obsolete: remove-me\ntags: [old-auto, manual]\n")
        .replace("# Filename", "# Renamed heading\nMy preface") + "\nMy ending\n";
    let replacement = document("status: active\ndescription: new\nstart-date: 2026-09-29\nvpt-open-flags: 3\ntags: [new-auto]\n").replace("[00:00] hello", "[00:00] changed");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &["old-auto".into()])
        .unwrap();
    for retained in [
        "status: complete # mine\n",
        "description: |\n  café\n  second line\n",
        "start-date: 2020-01-01\n",
        "custom: {nested: [one, two]}\n",
        "# keep this comment\n",
        "# Renamed heading\nMy preface",
        "\nMy ending\n",
    ] {
        assert!(rewritten.contains(retained), "{retained}");
    }
    assert!(!rewritten.contains("vpt-obsolete"));
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
        "---\n{{vpt-schema: 1, custom: 'café, 世界', vpt-obsolete: [old], vpt-recording: '{ID}', tags: [manual], vpt-stage: transcript}}\n---\n{BODY}"
    );
    let replacement = document("vpt-open-flags: 2\ntags: [invoice]\n");
    let rewritten = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &[])
        .unwrap();
    assert!(rewritten.contains("custom: 'café, 世界'"));
    assert!(!rewritten.contains("vpt-obsolete"));
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
    let source = document("vpt-engines:\n  - \"custom:test\"\nvpt-open-flags: 0\ntags: []\n");
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
            "custom: &model custom:test\nengines: &names [custom:test]\nvpt-engines: {value}\n"
        ));
        let replacement = document("vpt-engines: [custom:test]\n");
        let rewritten = Document::parse(&source)
            .unwrap()
            .rewrite(&replacement, &[])
            .unwrap();
        assert!(rewritten.contains("vpt-engines: [\"custom:test\"]"));
        assert!(rewritten.contains("custom: &model custom:test"));
    }
}
