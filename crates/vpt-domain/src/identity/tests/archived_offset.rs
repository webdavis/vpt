use super::{RecordingId, UtcInstant, UtcOffset};

fn named(timestamp: &str) -> RecordingId {
    RecordingId::parse(&format!("{timestamp}-4f3ab19c02de")).unwrap()
}

#[test]
fn eastern_archive_preserves_its_positive_fractional_hour_offset() {
    let id = named("2026-08-25T021736");
    assert_eq!(
        id.archived_offset(UtcInstant {
            secs: 1_787_604_456
        }),
        Some(UtcOffset { secs: 19_800 })
    );
}

#[test]
fn western_archive_preserves_its_negative_offset() {
    let id = named("2026-08-24T144736");
    assert_eq!(
        id.archived_offset(UtcInstant {
            secs: 1_787_604_456
        }),
        Some(UtcOffset { secs: -21_600 })
    );
}

#[test]
fn matching_archive_timestamp_has_zero_offset() {
    assert_eq!(
        named("1970-01-01T000000").archived_offset(UtcInstant { secs: 0 }),
        Some(UtcOffset { secs: 0 })
    );
}

#[test]
fn offset_inference_preserves_seconds_across_calendar_boundaries() {
    for (timestamp, captured, offset) in [
        ("1969-12-31T235959", 0, -1),
        ("1970-01-01T000000", -1, 1),
        ("2000-03-01T000001", 951_868_800, 1),
        ("0000-01-01T000000", -62_167_219_199, -1),
        ("9999-12-31T235959", 253_402_300_800, -1),
    ] {
        assert_eq!(
            named(timestamp).archived_offset(UtcInstant { secs: captured }),
            Some(UtcOffset { secs: offset }),
            "{timestamp}"
        );
    }
}

#[test]
fn offsets_one_second_inside_either_day_boundary_are_valid() {
    let id = named("1970-01-01T000000");
    for offset in [-86_399, 86_399] {
        assert_eq!(
            id.archived_offset(UtcInstant {
                secs: -i64::from(offset)
            }),
            Some(UtcOffset { secs: offset })
        );
    }
}

#[test]
fn offsets_at_and_beyond_either_day_boundary_are_refused() {
    let id = named("1970-01-01T000000");
    for captured in [-86_401, -86_400, 86_400, 86_401] {
        assert_eq!(id.archived_offset(UtcInstant { secs: captured }), None);
    }
}

#[test]
fn extreme_capture_times_cannot_overflow_or_wrap_into_valid_offsets() {
    let id = named("0000-01-01T000000");
    for captured in [i64::MIN, i64::MAX] {
        assert_eq!(id.archived_offset(UtcInstant { secs: captured }), None);
    }
    let id = named("1970-01-01T000000");
    for captured in [
        i64::from(i32::MIN),
        i64::from(i32::MAX),
        -4_294_967_296,
        -4_294_967_295,
        4_294_967_295,
        4_294_967_296,
    ] {
        assert_eq!(id.archived_offset(UtcInstant { secs: captured }), None);
    }
}
