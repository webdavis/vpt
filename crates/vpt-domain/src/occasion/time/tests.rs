use super::*;

#[test]
fn occasion_start_keeps_its_local_date_offset_and_exact_fraction() {
    let at = OccasionTime::parse("2026-09-30t23:30:12.123456789012300-06:00").unwrap();
    assert_eq!(at.as_str(), "2026-09-30T23:30:12.1234567890123-06:00");
    assert_eq!(at.local_timestamp(), "2026-09-30T233012");
    assert_eq!(at.date(), "2026-09-30");
    assert_eq!(at.unix_second().rfc3339(), "2026-10-01T05:30:12Z");
    let later = OccasionTime::parse("2026-10-01T05:30:12.1234567890124Z").unwrap();
    assert_eq!(at.cmp_instant(&later), Ordering::Less);
    let equivalent = OccasionTime::parse("2026-10-01T05:30:12.1234567890123+00:00").unwrap();
    assert_eq!(at.cmp_instant(&equivalent), Ordering::Equal);
    assert_eq!(equivalent.as_str(), "2026-10-01T05:30:12.1234567890123Z");
}

#[test]
fn zero_fraction_and_case_normalize_without_erasing_unknown_offset() {
    assert_eq!(
        OccasionTime::parse("2024-02-29t00:00:00.000z")
            .unwrap()
            .as_str(),
        "2024-02-29T00:00:00Z"
    );
    let unknown = OccasionTime::parse("2024-02-29T00:00:00-00:00").unwrap();
    assert_eq!(unknown.as_str(), "2024-02-29T00:00:00-00:00");
}

#[test]
fn leap_seconds_preserve_identity_and_order_between_adjacent_seconds() {
    let before = OccasionTime::parse("1990-12-31T23:59:59.999Z").unwrap();
    let leap = OccasionTime::parse("1990-12-31T15:59:60.1-08:00").unwrap();
    let after = OccasionTime::parse("1991-01-01T00:00:00Z").unwrap();
    assert_eq!(leap.local_timestamp(), "1990-12-31T155960");
    assert_eq!(before.cmp_instant(&leap), Ordering::Less);
    assert_eq!(leap.cmp_instant(&after), Ordering::Less);
    assert!(OccasionTime::parse("1990-12-31T12:00:60Z").is_err());
}

#[test]
fn invalid_calendar_clock_offset_and_unbounded_input_are_refused() {
    for value in [
        "",
        "2026-09-30",
        "2026-09-30T12:00:00",
        "2026-09-30 12:00:00Z",
        "2026-09-30T12:00:00.Z",
        "2026-09-30T12:00:00,1Z",
        "2026-09-30T24:00:00Z",
        "2026-09-30T12:60:00Z",
        "2026-09-30T12:00:61Z",
        "2023-02-29T12:00:00Z",
        "2026-09-31T12:00:00Z",
        "2026-09-30T12:00:00+24:00",
        "2026-09-30T12:00:00-00:60",
        "2026-09-30T12:00:00+0600",
        "2026-09-30T12:00:00Zextra",
        "2026-09-30T12:00:00é",
    ] {
        assert!(OccasionTime::parse(value).is_err(), "{value}");
    }
    assert_eq!(
        OccasionTime::parse(&"1".repeat(65_537)),
        Err(TimeError::CapacityExceeded)
    );
    for value in ["0000-01-01T00:00:00+23:59", "9999-12-31T23:59:59-23:59"] {
        assert!(OccasionTime::parse(value).is_ok());
    }
}
