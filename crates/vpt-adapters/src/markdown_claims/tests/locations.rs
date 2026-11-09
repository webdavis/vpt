use super::*;

#[test]
fn finding_locations_use_original_utf8_bytes_and_one_based_lines() {
    let source = "---\r\ntitle: hidden\r\n---\r\n# Heading\r\n\r\n- café [00:00-00:01]\r\n\r\nFirst sentence. Second sentence!\r\ncontinued claim\r\n";
    let parsed = extract(source).unwrap();
    assert_eq!(parsed.claims.len(), 4);
    for (claim, text, line) in [
        (&parsed.claims[0], "café [00:00-00:01]", 6),
        (&parsed.claims[1], "First sentence.", 8),
        (&parsed.claims[2], "Second sentence!", 8),
        (&parsed.claims[3], "continued claim", 9),
    ] {
        assert_eq!(claim.line(), line);
        assert_eq!(&source[claim.source_range()], text);
    }
}

#[test]
fn nested_claim_locations_follow_their_own_text_and_include_inline_markup_endings() {
    let source = "- **parent**\n  - child\n\nLast sentence.\n";
    let parsed = extract(source).unwrap();
    for (claim, text, line) in [
        (&parsed.claims[0], "parent**", 1),
        (&parsed.claims[1], "child", 2),
        (&parsed.claims[2], "Last sentence.", 4),
    ] {
        assert_eq!(claim.line(), line);
        assert_eq!(&source[claim.source_range()], text);
    }
}
