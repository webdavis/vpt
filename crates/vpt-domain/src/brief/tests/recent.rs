use super::*;

fn continues(id: RecordingId) -> Relation {
    Relation {
        kind: Kind::Continues,
        target: Target::Recording(id),
        state: State::Confirmed,
        provenance: Provenance::Manual,
        ranges: vec![],
    }
}

#[test]
fn recent_shared_tags_expand_transitively_then_rank_newest_first() {
    let occasion = occasion();
    let mut rows = vec![
        candidate(1, 100),
        candidate(2, 800_000),
        candidate(3, 900_000),
        candidate(4, 950_000),
    ];
    rows[0].tags = vec![
        tag("planning", tags::State::Confirmed),
        tag("bridge", tags::State::Confirmed),
    ];
    rows[1].tags = vec![
        tag("bridge", tags::State::Confirmed),
        tag("next", tags::State::Confirmed),
    ];
    rows[2].tags = vec![tag("next", tags::State::Confirmed)];
    rows[3].tags = vec![tag("next", tags::State::Suggested)];
    let result = select(&request(&occasion, &[], &rows)).unwrap();
    assert_eq!(result.selected, [0, 2, 1]);
    assert_eq!(
        result.decisions[1].matched.as_ref().unwrap().selector,
        Selector::Recent
    );
    assert_eq!(result.rejected_notes, 1);
    let mut capped = request(&occasion, &[], &rows);
    capped.options.max_notes = 2;
    let result = select(&capped).unwrap();
    assert_eq!(result.selected, [0, 2]);
    assert_eq!(result.omitted_notes, 1);
}

#[test]
fn continuation_chains_cross_older_or_artifactless_nodes_but_select_only_recent_transcripts() {
    let occasion = occasion();
    let mut rows = vec![
        candidate(1, -30_000_000),
        candidate(2, -20_000_000),
        candidate(3, 900_000),
        candidate(4, 950_000),
    ];
    rows[0].tags.push(tag("planning", tags::State::Confirmed));
    rows[1].source = None;
    let target = rows[0].recording.clone();
    rows[1].relations.push(continues(target));
    let target = rows[1].recording.clone();
    rows[2].relations.push(continues(target));
    let target = rows[2].recording.clone();
    rows[3].relations.push(continues(target));
    rows[3].relations[0].state = State::Suggested;
    let result = select(&request(&occasion, &[], &rows)).unwrap();
    assert_eq!(result.selected, [0, 2]);
    assert_eq!(result.rejected_notes, 2);
}

#[test]
fn recent_lookback_is_inclusive_at_the_lower_bound_and_excludes_future_captures() {
    let occasion = occasion();
    let lower = 1_000_000 - 180 * 86_400;
    let mut rows = vec![
        candidate(1, 100),
        candidate(2, lower),
        candidate(3, lower - 1),
        candidate(4, 1_000_001),
    ];
    rows[0].tags = vec![
        tag("planning", tags::State::Confirmed),
        tag("shared", tags::State::Confirmed),
    ];
    for row in &mut rows[1..] {
        row.tags.push(tag("shared", tags::State::Confirmed));
    }
    let result = select(&request(&occasion, &[], &rows)).unwrap();
    assert_eq!(result.selected, [0, 1]);
    assert_eq!(
        result
            .decisions
            .iter()
            .find(|d| d.recording == rows[2].recording)
            .unwrap()
            .rejected,
        Some(Rejection::OutsideLookback)
    );
    assert_eq!(
        result
            .decisions
            .iter()
            .find(|d| d.recording == rows[3].recording)
            .unwrap()
            .rejected,
        Some(Rejection::FutureCapture)
    );
}

#[test]
fn recent_candidates_need_a_confirmed_anchor_even_when_they_form_a_cycle() {
    let mut occasion = occasion();
    occasion.tags.clear();
    let mut rows = vec![candidate(1, 100), candidate(2, 200)];
    let target = rows[1].recording.clone();
    rows[0].relations.push(continues(target));
    let target = rows[0].recording.clone();
    rows[1].relations.push(continues(target));
    for row in &mut rows {
        row.tags.push(tag("shared", tags::State::Confirmed));
    }
    assert!(
        select(&request(&occasion, &[], &rows))
            .unwrap()
            .selected
            .is_empty()
    );
}
