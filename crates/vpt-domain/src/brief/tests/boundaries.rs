use super::*;

#[test]
fn conflicting_duplicate_recordings_and_invalid_sources_are_refused() {
    let occasion = occasion();
    let mut rows = vec![candidate(1, 100)];
    rows[0].tags.push(tag("planning", tags::State::Confirmed));
    let mut duplicate = rows[0].clone();
    duplicate.source.as_mut().unwrap().digest = Sha256Digest([7; 32]);
    rows.push(duplicate);
    assert_eq!(
        select(&request(&occasion, &[], &rows)).unwrap_err(),
        BriefError::ConflictingRecording
    );
    rows.pop();
    for case in 0..3 {
        let mut broken = rows.clone();
        let source = broken[0].source.as_mut().unwrap();
        match case {
            0 => source.transcript.segments[0].start = f64::NAN,
            1 => source.note = "/notes/../private.md".into(),
            _ => source.transcript.segments[0].end = (1u64 << 53) as f64 + 2.0,
        }
        assert!(
            assemble(&request(&occasion, &[], &broken)).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn provider_labels_and_confirmed_tags_share_normalization_and_zero_caps_are_explicit() {
    let mut occasion = occasion();
    occasion.tags = vec!["Private Planning".into()];
    let mut rows = vec![candidate(1, 100)];
    rows[0]
        .tags
        .push(tag("private_planning", tags::State::Confirmed));
    let mut request = request(&occasion, &[], &rows);
    assert_eq!(select(&request).unwrap().selected, [0]);
    request.options.max_notes = 0;
    let pack = assemble(&request).unwrap();
    assert!(pack.items.is_empty());
    assert_eq!(pack.selection.omitted_notes, 1);
    request.options.max_notes = 12;
    request.options.max_spans_per_note = 0;
    let pack = assemble(&request).unwrap();
    assert!(pack.items.is_empty());
    assert_eq!(pack.omitted_spans, 1);
}

#[test]
fn participant_confirmation_preserves_shell_quoting_and_unicode_matching() {
    let mut occasion = occasion();
    occasion.participants[0].name = "Private O'Neil $(touch sentinel)".into();
    let result = select(&request(&occasion, &[], &[])).unwrap();
    assert_eq!(
        result.participants[0].confirmation.as_deref(),
        Some("vpt confirm --term 'Private O'\\''Neil $(touch sentinel)'")
    );
    occasion.participants[0].name = "ＳＴＲＡＳＳＥ".into();
    let terms = [term("Straße", "/index/person.md")];
    let mut row = candidate(1, 100);
    row.relations.push(mention("/index/person.md"));
    assert_eq!(
        select(&request(&occasion, &terms, &[row]))
            .unwrap()
            .participants[0]
            .term,
        TermState::Confirmed
    );
}

#[test]
fn limits_and_lookback_arithmetic_refuse_excess_without_panicking() {
    let occasion = occasion();
    let rows = [candidate(1, 100)];
    let terms = vec![
        Term {
            text: String::new(),
            target: TermTarget::Missing
        };
        65_537
    ];
    let mut request = request(&occasion, &terms, &rows);
    assert_eq!(select(&request).unwrap_err(), BriefError::CapacityExceeded);
    request.terms = &[];
    request.options = Options {
        lookback_days: u32::MAX,
        ..Default::default()
    };
    request.now = UtcInstant { secs: i64::MIN };
    assert_eq!(
        select(&request).unwrap().decisions[0].rejected,
        Some(Rejection::FutureCapture)
    );
}

#[test]
fn large_counts_select_only_available_notes_and_spans() {
    let occasion = occasion();
    let mut rows = [candidate(1, 100)];
    rows[0].tags.push(tag("planning", tags::State::Confirmed));
    let mut request = request(&occasion, &[], &rows);
    for value in [65_537, u32::MAX as usize] {
        request.options.max_notes = value;
        request.options.max_spans_per_note = value;
        let pack = assemble(&request).unwrap();
        assert_eq!(pack.selection.selected, [0]);
        assert_eq!(pack.sources.len(), 1);
        assert_eq!(pack.items.len(), 1);
        assert_eq!(pack.omitted_spans, 0);
        assert_eq!(pack.selection.omitted_notes, 0);
    }
}

#[test]
fn streamed_input_budget_limits_candidate_count_and_total_text() {
    let mut empty = candidate(1, 1);
    empty.source = None;
    let mut budget = InputBudget::default();
    for _ in 0..65_536 {
        budget.candidate(&empty).unwrap();
    }
    assert_eq!(budget.candidate(&empty), Err(BriefError::CapacityExceeded));
    let mut row = candidate(1, 1);
    row.source.as_mut().unwrap().transcript.segments[0].text = "x".repeat(65_536);
    let mut budget = InputBudget::default();
    let mut accepted = 0;
    while budget.candidate(&row).is_ok() {
        accepted += 1;
        assert!(accepted < 1024);
    }
    assert!(accepted > 1);
}
