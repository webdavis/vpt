use super::*;

const LINKS: &str = "<!-- vpt:links start -->\nold links\n<!-- vpt:links end -->";
const CONTENT: &str = "<!-- vpt:content start -->\nold content\n<!-- vpt:content end -->";

#[test]
fn rewriting_only_replaces_region_contents_and_preserves_every_other_byte() {
    let source = format!(
        "---\nstatus: complete\n---\n# renamed title\n\nPersonal preface  \n{LINKS}\n\nMy middle text\n{CONTENT}\n\nMy ending\n"
    );
    let regions = Regions::parse(&source).unwrap();
    assert_eq!(regions.content(), "old content\n");
    let rewritten = regions.rewrite("new links\n", "new content\n").unwrap();
    assert_eq!(
        rewritten,
        source
            .replace("old links\n", "new links\n")
            .replace("old content\n", "new content\n")
    );
    assert_eq!(
        Regions::parse(&rewritten).unwrap().content(),
        "new content\n"
    );
}

#[test]
fn link_only_changes_leave_exact_content_bytes_unchanged() {
    let source = format!("{LINKS}\n\n{CONTENT}\n");
    let regions = Regions::parse(&source).unwrap();
    let rewritten = regions.rewrite("- New link\n", regions.content()).unwrap();
    assert_eq!(
        Regions::parse(&rewritten).unwrap().content(),
        regions.content()
    );
}

#[test]
fn either_region_order_is_supported_and_existing_marker_line_endings_survive() {
    let source = format!("{CONTENT}\n\n{LINKS}").replace('\n', "\r\n");
    let regions = Regions::parse(&source).unwrap();
    assert_eq!(regions.content(), "old content\r\n");
    assert_eq!(
        regions.rewrite("", "replacement").unwrap(),
        "<!-- vpt:content start -->\r\nreplacement\n<!-- vpt:content end -->\r\n\r\n<!-- vpt:links start -->\r\n<!-- vpt:links end -->"
    );
}

#[test]
fn missing_duplicate_reversed_nested_or_nonstandalone_markers_are_refused() {
    let valid = format!("{LINKS}\n{CONTENT}\n");
    for invalid in [
        valid.replace("<!-- vpt:links end -->", ""),
        format!("{valid}{CONTENT}"),
        valid.replace("<!-- vpt:content start -->", "<!-- vpt:content end -->"),
        valid.replace("<!-- vpt:links start -->", "prefix <!-- vpt:links start -->"),
        valid.replace("<!-- vpt:links end -->", "<!-- vpt:links end --> trailing"),
        "<!-- vpt:links start -->\n<!-- vpt:content start -->\n<!-- vpt:links end -->\n<!-- vpt:content end -->\n".into(),
        "<!-- vpt:links end -->\n<!-- vpt:links start -->\n<!-- vpt:content start -->\n<!-- vpt:content end -->\n".into(),
    ] {
        assert!(matches!(Regions::parse(&invalid), Err(RenderError::Markers)), "{invalid}");
    }
}

#[test]
fn replacement_cannot_introduce_managed_markers() {
    let source = format!("{LINKS}\n{CONTENT}\n");
    let regions = Regions::parse(&source).unwrap();
    for token in [
        "<!-- vpt:links start -->",
        "<!-- vpt:links end -->",
        "<!-- vpt:content start -->",
        "<!-- vpt:content end -->",
    ] {
        assert!(matches!(
            regions.rewrite(token, "safe"),
            Err(RenderError::Markers)
        ));
        assert!(matches!(
            regions.rewrite("safe", token),
            Err(RenderError::Markers)
        ));
    }
}

#[test]
fn empty_and_nonterminated_replacements_leave_valid_delimiters() {
    let source = format!("{LINKS}\n{CONTENT}\n");
    let regions = Regions::parse(&source).unwrap();
    let rewritten = regions.rewrite("", "new text").unwrap();
    assert_eq!(Regions::parse(&rewritten).unwrap().content(), "new text\n");
    assert!(rewritten.contains("<!-- vpt:links start -->\n<!-- vpt:links end -->"));
}

#[test]
fn oversized_source_or_replacement_is_refused_before_copying() {
    let source = "x".repeat(super::super::MAX_RENDERED + 1);
    assert!(matches!(
        Regions::parse(&source),
        Err(RenderError::CapacityExceeded)
    ));
    let valid = format!("{LINKS}\n{CONTENT}\n");
    let regions = Regions::parse(&valid).unwrap();
    assert!(matches!(
        regions.rewrite("", &source),
        Err(RenderError::CapacityExceeded)
    ));
}
