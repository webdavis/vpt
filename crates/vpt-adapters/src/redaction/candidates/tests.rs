use super::super::*;
use vpt_domain::redaction::Options;
#[test]
fn candidates_come_from_surviving_decoded_text_and_keep_release_line_numbers() {
    let source = "First Al&#105;ce met **Zoë** and [Béla](https://Private). Next Léo\nwith Zoë. <span>HiddenName</span>\n\n## Private\n\nkeep HiddenSecret\n\n## Public\n\nStart with Béla.\n";
    let mut redactor = Redactor::new(
        &["Alice"],
        &[],
        Options {
            mask: "[Hidden! {n}]".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let content = redact_content(
        source,
        &mut redactor,
        &ContentOptions {
            deny_sections: vec!["Private".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        content
            .candidates
            .iter()
            .map(|c| (c.token.as_str(), c.line))
            .collect::<Vec<_>>(),
        [("Zoë", 1), ("Béla", 1), ("Léo", 1), ("Zoë", 2), ("Béla", 6)]
    );
    assert!(!format!("{:?}", content.candidates).contains("Zoë"));
    let released = document::assemble(content, &document::Header::default(), &redactor).unwrap();
    assert_eq!(
        released
            .candidates
            .iter()
            .map(|c| c.line)
            .collect::<Vec<_>>(),
        [5, 5, 5, 6, 10]
    );
}
#[test]
fn surviving_title_and_source_candidates_are_reported_at_their_yaml_lines() {
    let mut redactor = Redactor::new(&[], &[], Options::default()).unwrap();
    let content =
        redact_content("Talk with Zoë", &mut redactor, &ContentOptions::default()).unwrap();
    let released = document::assemble(
        content,
        &document::Header {
            title: Some("Meeting with Béla".into()),
            source: "Extract from Léo".into(),
            date: None,
        },
        &redactor,
    )
    .unwrap();
    assert_eq!(
        released
            .candidates
            .iter()
            .map(|c| (c.token.as_str(), c.line))
            .collect::<Vec<_>>(),
        [("Béla", 2), ("Léo", 3), ("Zoë", 6)]
    );
}

#[test]
fn candidate_count_is_bounded_across_blocks_and_errors_do_not_include_surviving_text() {
    let source = format!(
        "Start {}\n\nStart {}",
        "Private ".repeat(32_769),
        "Private ".repeat(32_769)
    );
    let error = redact_content(
        &source,
        &mut Redactor::new(&[], &[], Options::default()).unwrap(),
        &ContentOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error, Error::Capacity);
    assert!(!format!("{error:?}").contains("Private"));
}
