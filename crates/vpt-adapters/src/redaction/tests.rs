use super::*;
use vpt_domain::redaction::Options;
fn run(source: &str, options: ContentOptions) -> Content {
    redact_content(
        source,
        &mut Redactor::new(&["Alice"], &["Atlas"], Options::default()).unwrap(),
        &options,
    )
    .unwrap()
}
pub(super) fn visible(markdown: &str) -> String {
    use pulldown_cmark::{Event, Parser};
    Parser::new(markdown)
        .filter_map(|e| match e {
            Event::Text(t) | Event::Code(t) => Some(t.to_string()),
            Event::SoftBreak | Event::HardBreak => Some("\n".into()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
#[test]
fn decoded_entities_and_inline_markup_cannot_split_a_sensitive_name() {
    let output = run(
        "Al&#105;ce meets **At**las and `Al&#105;ce`.\n",
        ContentOptions::default(),
    );
    assert_eq!(
        visible(&output.text),
        "[term 1] meets [note 2] and [term 3]."
    );
    assert_eq!(output.mappings.len(), 3);
    assert!(!format!("{output:?}").contains("Alice"));
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].line, 1);
    assert_eq!(
        output.lines[0].source,
        0.."Al&#105;ce meets **At**las and `Al&#105;ce`.\n".len()
    );
}
#[test]
fn links_images_wikilinks_and_reference_definitions_keep_only_visible_labels() {
    let output = run(
        "[Alice](https://PRIVATE.example) ![Atlas](private.png) [[Private target|public alias]] [safe][id]\n\n[id]: https://hidden.example \"PRIVATE title\"\n",
        ContentOptions::default(),
    );
    let text = visible(&output.text);
    assert!(
        text.contains("[term 1] [note 2] public alias safe"),
        "{text}"
    );
    for hidden in ["PRIVATE", "Private target", "private.png", "hidden.example"] {
        assert!(!output.text.contains(hidden));
    }
}
#[test]
fn raw_html_comments_and_metadata_are_discarded_with_hidden_element_content() {
    let source = "---\nvptRecording: PRIVATE\ntitle: PRIVATE\n---\n\nPublic <span style='display:none'>PRIVATE <b>hidden</b></span> visible <!-- PRIVATE comment -->.\n\n<script>PRIVATE script</script>\n\nAfter <br> line.\n";
    let output = run(source, ContentOptions::default());
    assert!(!output.text.contains("PRIVATE"));
    assert!(!output.text.contains("hidden"));
    assert!(visible(&output.text).contains("Public  visible ."));
    assert!(visible(&output.text).contains("After  line."));
}
#[test]
fn denied_sections_include_children_and_end_at_the_next_peer_heading() {
    let source = "# Public\n\nkeep\n\n## Private\n\nsecret\n\n### Child\n\nsecret child\n\n## Next\n\nretain\n";
    let output = run(
        source,
        ContentOptions {
            deny_sections: vec!["private".into()],
            ..Default::default()
        },
    );
    assert!(!output.text.contains("secret"));
    assert!(!output.text.contains("Child"));
    assert!(output.text.contains("Next"));
    assert!(output.text.contains("retain"));
}
#[test]
fn timecodes_are_removed_by_default_and_retained_only_when_requested() {
    let source = "[01:23] Alice cited [00:01-00:03]. Literal [not:time] stays.\n";
    let output = run(source, ContentOptions::default());
    let text = visible(&output.text);
    assert!(!text.contains("01:23"));
    assert!(!text.contains("00:01"));
    assert!(text.contains("[not:time]"));
    let kept = run(
        source,
        ContentOptions {
            keep_timecodes: true,
            ..Default::default()
        },
    );
    assert!(visible(&kept.text).contains("[01:23]"));
}
#[test]
fn reescaped_retained_text_cannot_reintroduce_links_or_html() {
    let output = run(
        "Literal &lt;img src=x&gt; and &#91;click&#93;(secret) and &amp;.\n",
        ContentOptions::default(),
    );
    use pulldown_cmark::{Event, Parser, Tag};
    assert!(!Parser::new(&output.text).any(|e| matches!(
        e,
        Event::Html(_) | Event::InlineHtml(_) | Event::Start(Tag::Link { .. } | Tag::Image { .. })
    )));
    assert!(visible(&output.text).contains("<img src=x>"));
}

#[test]
fn fenced_code_entities_are_decoded_before_masking_and_hidden_html_never_leaks_across_blocks() {
    let code = run(
        "```text\nAl&#105;ce &amp; Atlas\n```\n",
        ContentOptions::default(),
    );
    assert_eq!(visible(&code.text), "[term 1] & [note 2]");
    let hidden = run(
        "Visible <span>PRIVATE\n\nPRIVATE hidden paragraph</span> after.\n",
        ContentOptions::default(),
    );
    assert!(!hidden.text.contains("PRIVATE"));
    assert!(visible(&hidden.text).contains("after."));
}

#[test]
fn malformed_deep_html_and_oversized_sections_refuse_without_private_errors() {
    let source = format!(
        "text {}PRIVATE{}",
        "<span>".repeat(129),
        "</span>".repeat(129)
    );
    let error = redact_content(
        &source,
        &mut Redactor::new(&[], &[], Options::default()).unwrap(),
        &ContentOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error, Error::Capacity);
    let options = ContentOptions {
        deny_sections: vec!["PRIVATE".into(); 257],
        ..Default::default()
    };
    let error = redact_content(
        "text",
        &mut Redactor::new(&[], &[], Options::default()).unwrap(),
        &options,
    )
    .unwrap_err();
    assert_eq!(error, Error::Capacity);
}
