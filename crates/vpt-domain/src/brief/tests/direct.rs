use super::*;

#[test]
fn confirmed_participant_title_term_and_tag_matches_rank_before_capture_time() {
    let occasion = occasion();
    let terms = [
        term("Private Person", "/index/person.md"),
        term("Private project", "/index/project.md"),
    ];
    let mut rows = vec![
        candidate(1, 100),
        candidate(2, 200),
        candidate(3, 400),
        candidate(4, 300),
    ];
    rows[0].relations.push(mention("/index/person.md"));
    rows[1].relations.push(mention("/index/project.md"));
    rows[2].tags.push(tag("planning", tags::State::Confirmed));
    rows[3].relations.push(mention("/index/person.md"));
    let result = select(&request(&occasion, &terms, &rows)).unwrap();
    assert_eq!(result.selected, [3, 0, 1, 2]);
    assert_eq!(result.participants[0].term, TermState::Confirmed);
    assert_eq!(
        result
            .decisions
            .iter()
            .map(|d| d.matched.as_ref().unwrap().selector)
            .collect::<Vec<_>>(),
        [
            Selector::Participant,
            Selector::Participant,
            Selector::Term,
            Selector::Tag
        ]
    );
    assert!(!format!("{result:?}").contains("Private"));
}

#[test]
fn unknown_or_ambiguous_participant_never_selects_a_note_and_has_a_fix() {
    let mut occasion = occasion();
    occasion.title = "Unrelated".into();
    occasion.tags.clear();
    let mut row = candidate(1, 100);
    row.relations.push(mention("/index/person.md"));
    let rows = [row];
    let missing = select(&request(&occasion, &[], &rows)).unwrap();
    assert!(missing.selected.is_empty());
    assert_eq!(missing.participants[0].term, TermState::Unconfirmed);
    assert_eq!(
        missing.participants[0].confirmation.as_deref(),
        Some("vpt confirm --term 'Private Person'")
    );
    for target in [TermTarget::Missing, TermTarget::Ambiguous] {
        let terms = [Term {
            text: "Private Person".into(),
            target,
        }];
        let result = select(&request(&occasion, &terms, &rows)).unwrap();
        assert!(result.selected.is_empty());
        assert!(result.participants[0].matched_note.is_none());
        assert!(result.participants[0].confirmation.is_none());
    }
}

#[test]
fn missing_artifacts_unconfirmed_tags_and_suggested_mentions_are_rejected() {
    let occasion = occasion();
    let terms = [term("Private Person", "/index/person.md")];
    let mut rows = vec![candidate(1, 100), candidate(2, 100), candidate(3, 100)];
    rows[0].source = None;
    rows[0].tags.push(tag("planning", tags::State::Confirmed));
    rows[1].tags.push(tag("planning", tags::State::Suggested));
    rows[2].relations.push(mention("/index/person.md"));
    rows[2].relations[0].state = State::Suggested;
    let result = select(&request(&occasion, &terms, &rows)).unwrap();
    assert!(result.selected.is_empty());
    assert_eq!(result.rejected_notes, 3);
    assert_eq!(
        result.decisions[0].rejected,
        Some(Rejection::MissingTranscript)
    );
}

#[test]
fn deduplication_precedes_note_limit_and_reports_all_omissions() {
    let mut occasion = occasion();
    occasion.participants.clear();
    let mut rows = vec![candidate(1, 100), candidate(2, 200), candidate(3, 300)];
    for row in &mut rows {
        row.tags.push(tag("planning", tags::State::Confirmed));
    }
    rows.insert(1, rows[0].clone());
    let mut request = request(&occasion, &[], &rows);
    request.options.max_notes = 2;
    let result = select(&request).unwrap();
    assert_eq!(result.selected, [3, 2]);
    assert_eq!(result.decisions.len(), 3);
    assert_eq!(result.omitted_notes, 1);
    assert_eq!(result.rejected_notes, 0);
}
