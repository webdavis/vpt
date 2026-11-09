use super::*;
use crate::{digest::Sha256Digest, time::UtcOffset};

fn id(seed: u8, at: i64) -> RecordingId {
    RecordingId::derive(
        UtcInstant { secs: at },
        UtcOffset { secs: 0 },
        &Sha256Digest([seed; 32]),
    )
    .unwrap()
}
fn capture(id: &RecordingId, at: i64, duration_secs: u64) -> Capture<'_> {
    Capture {
        id,
        at: UtcInstant { secs: at },
        duration_secs,
    }
}

#[test]
fn nearest_predecessor_is_chosen_by_capture_instant_not_input_order_or_identity() {
    let (first, next, later, current) = (id(9, 100), id(2, 300), id(1, 800), id(3, 700));
    let rows = [
        capture(&later, 800, 1),
        capture(&next, 300, 20),
        capture(&current, 700, 5),
        capture(&first, 100, 1),
    ];
    assert_eq!(continues(capture(&current, 700, 5), &rows, 60), Some(next));
}

#[test]
fn an_overlapping_predecessor_never_falls_back_to_an_older_recording() {
    let (old, overlapping, current) = (id(1, 100), id(2, 300), id(3, 400));
    let rows = [capture(&old, 100, 1), capture(&overlapping, 300, 200)];
    assert_eq!(continues(capture(&current, 400, 1), &rows, 60), None);
}

#[test]
fn identity_breaks_capture_ties_before_the_gap_is_checked() {
    let (older, low, high, current) = (id(1, 100), id(2, 300), id(3, 300), id(4, 400));
    let rows = [
        capture(&older, 100, 1),
        capture(&high, 300, 200),
        capture(&low, 300, 1),
    ];
    assert_eq!(continues(capture(&current, 400, 1), &rows, 60), None);
    let rows = [capture(&high, 300, 1), capture(&low, 300, 1)];
    assert_eq!(continues(capture(&current, 400, 1), &rows, 60), Some(high));
}

#[test]
fn ending_must_precede_start_and_the_gap_boundary_is_inclusive() {
    let (previous, current) = (id(1, 100), id(2, 200));
    for (end, window, expected) in [
        (140, 1, true),
        (139, 1, false),
        (199, 0, false),
        (200, 60, false),
    ] {
        let actual = continues(
            capture(&current, 200, 1),
            &[capture(&previous, 100, end - 100)],
            window,
        );
        assert_eq!(actual.is_some(), expected);
    }
}

#[test]
fn extreme_durations_and_instants_cannot_wrap_into_a_continuation() {
    let (previous, current) = (id(1, 100), id(2, 200));
    assert_eq!(
        continues(
            capture(&current, i64::MAX, 1),
            &[capture(&previous, i64::MAX - 1, u64::MAX)],
            u32::MAX
        ),
        None
    );
    assert_eq!(
        continues(
            capture(&current, i64::MAX, 1),
            &[capture(&previous, i64::MIN, 0)],
            u32::MAX
        ),
        None
    );
    assert_eq!(continues(capture(&current, 200, 1), &[], 60), None);
}

#[test]
fn relation_kinds_are_a_closed_set() {
    use crate::relations::Kind;
    for kind in [
        Kind::Continues,
        Kind::Mentions,
        Kind::Related,
        Kind::Supersedes,
    ] {
        assert_eq!(Kind::parse(kind.as_str()), Some(kind));
    }
    for unknown in ["", "RELATED", "references", "private"] {
        assert_eq!(Kind::parse(unknown), None);
    }
}

#[test]
fn local_identity_time_zones_do_not_change_predecessor_order() {
    let id_at = |seed, secs, offset| {
        RecordingId::derive(
            UtcInstant { secs },
            UtcOffset { secs: offset },
            &Sha256Digest([seed; 32]),
        )
        .unwrap()
    };
    let older = id_at(1, 1000, 36000);
    let newer = id_at(2, 1100, -36000);
    let current = id_at(3, 1200, 0);
    assert!(older.as_str() > newer.as_str());
    assert_eq!(
        continues(
            capture(&current, 1200, 1),
            &[capture(&older, 1000, 1), capture(&newer, 1100, 1)],
            60
        ),
        Some(newer)
    );
}
