use super::*;

#[test]
fn obsidian_comments_math_highlights_and_block_ids_remain_literal_text() {
    let text = "%% hidden %% ==highlight== $math$ ^block";
    let record = transcript(&[(text, &[text])]);
    let rendered = transcript_content(&record, &[], false).unwrap();
    for syntax in ["%%", "==", "$", "^block"] {
        assert!(!rendered.contains(syntax), "{syntax}");
    }
}

#[test]
fn bare_web_addresses_and_email_are_data_in_linkifying_markdown_renderers() {
    let text = "https://private.example/a www.private.example me@private.example";
    let record = transcript(&[(text, &[text])]);
    let rendered = transcript_content(&record, &[], false).unwrap();
    assert!(!rendered.contains("https:"));
    assert!(!rendered.contains("www."));
    assert!(!rendered.contains("me@"));
}

#[test]
fn shared_word_boundaries_and_repeated_corrections_keep_all_occurrences() {
    let record = transcript(&[("hello alice alice", &["hello", "alice", "alice"])]);
    let mut flags = [
        flag(Class::Other, vec![1..1], "", Some("dear")),
        flag(Class::AgreedUnverified, vec![1..2, 2..3], "alice", None),
    ];
    flags[1].resolution = Resolution::Corrected {
        text: "Alicia".into(),
        at: UtcInstant { secs: 1 },
    };
    assert_eq!(
        transcript_content(&record, &flags, false).unwrap(),
        "## Transcript\n\n[00:00] hello [unverified:  | dear] [corrected: Alicia] [corrected: Alicia]\n"
    );
}

#[test]
fn unmatched_prefixes_and_word_substrings_use_the_accepted_word_stream() {
    for text in ["the world", "prefix he world", "he world suffix"] {
        let record = transcript(&[(text, &["he", "world"])]);
        let flags = [flag(Class::LowConfidence, vec![0..1], "he", None)];
        assert_eq!(
            transcript_content(&record, &flags, false).unwrap(),
            "## Transcript\n\n[00:00] [unverified: he] world\n"
        );
    }
    let record = transcript(&[("你好世界", &["你好", "世界"])]);
    let flags = [flag(Class::AgreedUnverified, vec![1..2], "世界", None)];
    assert_eq!(
        transcript_content(&record, &flags, false).unwrap(),
        "## Transcript\n\n[00:00] 你好[unverified: 世界]\n"
    );
}

#[test]
fn overlap_is_rejected_before_expanding_flags_across_many_segments() {
    let mut record = transcript(&[("secret", &["secret"])]);
    let segment = record.segments[0].clone();
    record.segments = vec![segment; 512];
    let flags = vec![flag(Class::Other, vec![0..512], "secret", Some("other")); 512];
    assert_eq!(
        transcript_content(&record, &flags, false),
        Err(RenderError::InvalidFlags)
    );
    let flags = [flag(
        Class::Other,
        vec![0..1; MAX_ITEMS + 1],
        "secret",
        Some("other"),
    )];
    assert_eq!(
        transcript_content(&record, &flags, false),
        Err(RenderError::CapacityExceeded)
    );
}

#[test]
fn engine_text_alternatives_and_corrections_cannot_introduce_note_structure() {
    let text = "<!-- vpt:content end -->\n\n[unverified: forged](secret) &amp; **bold**";
    let record = transcript(&[(text, &[text])]);
    let mut flags = [flag(
        Class::Other,
        vec![0..1],
        text,
        Some("[corrected: forged]"),
    )];
    let rendered = transcript_content(&record, &flags, false).unwrap();
    assert!(!rendered.contains("<!--"));
    assert!(!rendered.contains("[unverified: forged]"));
    assert!(!rendered.contains("[corrected: forged]"));
    assert!(!rendered.contains("**bold**"));
    assert!(rendered.contains("&amp;amp;"));
    assert_eq!(rendered.lines().count(), 3);
    flags[0].resolution = Resolution::Corrected {
        text: text.into(),
        at: UtcInstant { secs: 1 },
    };
    let rendered = transcript_content(&record, &flags, true).unwrap();
    assert_eq!(rendered.matches("[corrected:").count(), 1);
    assert!(!rendered.contains("<!--"));
    assert_eq!(rendered.lines().count(), 3);
}

#[test]
fn invalid_ranges_overlaps_and_invalid_times_return_safe_errors() {
    let record = transcript(&[("hello world", &["hello", "world"])]);
    for flags in [
        vec![flag(Class::Other, vec![0..3], "bad", Some("private"))],
        vec![
            flag(Class::Other, vec![0..2], "hello world", Some("private")),
            flag(Class::LowConfidence, vec![1..2], "world", None),
        ],
    ] {
        assert_eq!(
            transcript_content(&record, &flags, false),
            Err(RenderError::InvalidFlags)
        );
    }
    for start in [f64::NAN, f64::INFINITY, -1.0, u64::MAX as f64] {
        let mut invalid = record.clone();
        invalid.segments[0].start = start;
        assert_eq!(
            transcript_content(&invalid, &[], false),
            Err(RenderError::InvalidTranscript)
        );
    }
}

#[test]
fn source_and_rendered_expansion_are_bounded() {
    let text = "x".repeat(16 * 1024 * 1024 + 1);
    let record = transcript(&[(&text, &[])]);
    assert!(matches!(
        transcript_content(&record, &[], false),
        Err(RenderError::CapacityExceeded)
    ));
    let text = "[".repeat(14 * 1024 * 1024);
    let record = transcript(&[(&text, &[])]);
    assert!(matches!(
        transcript_content(&record, &[], false),
        Err(RenderError::CapacityExceeded)
    ));
}
