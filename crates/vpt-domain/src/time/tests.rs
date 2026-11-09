use super::*;

#[test]
fn epoch_and_negative_instant_render() {
    assert_eq!(UtcInstant { secs: 0 }.rfc3339(), "1970-01-01T00:00:00Z");
    assert_eq!(UtcInstant { secs: -1 }.rfc3339(), "1969-12-31T23:59:59Z");
}

#[test]
fn offsets_shift_civil_time_and_keep_their_sign() {
    let captured = UtcInstant {
        secs: 1_787_604_456,
    };
    assert_eq!(captured.rfc3339(), "2026-08-24T20:47:36Z");
    assert_eq!(
        captured.rfc3339_with(UtcOffset { secs: -21_600 }),
        "2026-08-24T14:47:36-06:00"
    );
    assert_eq!(
        captured.rfc3339_with(UtcOffset { secs: 19_800 }),
        "2026-08-25T02:17:36+05:30"
    );
    assert_eq!(UtcOffset { secs: i32::MIN }.label(), "-596523:14");
}

#[test]
fn leap_day_and_negative_years_round_trip() {
    let leap = UtcInstant {
        secs: 1_709_164_800,
    }
    .civil(UtcOffset { secs: 0 });
    assert_eq!((leap.year, leap.month, leap.day), (2024, 2, 29));
    let captured = UtcInstant {
        secs: -62_167_219_200,
    };
    let civil = captured.civil(UtcOffset { secs: 0 });
    assert_eq!((civil.year, civil.month, civil.day), (0, 1, 1));
    assert_eq!(civil.instant(UtcOffset { secs: 0 }), Some(captured));
}

#[test]
fn civil_round_trips_at_positive_and_negative_offsets() {
    let captured = UtcInstant {
        secs: 1_787_604_456,
    };
    for offset in [
        UtcOffset { secs: -21_600 },
        UtcOffset { secs: 19_800 },
        UtcOffset { secs: 0 },
    ] {
        assert_eq!(captured.civil(offset).instant(offset), Some(captured));
    }
}

#[test]
fn extreme_instants_convert_and_out_of_range_civil_is_refused() {
    let late = UtcInstant { secs: i64::MAX }.civil(UtcOffset { secs: 50_400 });
    let early = UtcInstant { secs: i64::MIN }.civil(UtcOffset { secs: -50_400 });
    assert!(late.year > early.year);
    assert_eq!(
        late.instant(UtcOffset { secs: 50_400 }),
        Some(UtcInstant { secs: i64::MAX })
    );
    assert_eq!(
        early.instant(UtcOffset { secs: -50_400 }),
        Some(UtcInstant { secs: i64::MIN })
    );
    assert_eq!(late.instant(UtcOffset { secs: -50_400 }), None);
}

#[test]
fn gregorian_calendar_and_clock_bounds_are_validated() {
    let base = Civil {
        year: 2024,
        month: 2,
        day: 29,
        hour: 23,
        minute: 59,
        second: 59,
    };
    assert!(base.is_valid());
    assert!(!Civil { year: 2023, ..base }.is_valid());
    assert!(Civil { year: 2000, ..base }.is_valid());
    assert!(!Civil { year: 1900, ..base }.is_valid());
    assert!(
        !Civil {
            month: 4,
            day: 31,
            ..base
        }
        .is_valid()
    );
    assert!(!Civil { month: 0, ..base }.is_valid());
    assert!(!Civil { month: 13, ..base }.is_valid());
    assert!(!Civil { day: 0, ..base }.is_valid());
    assert!(!Civil { hour: 24, ..base }.is_valid());
    assert!(!Civil { minute: 60, ..base }.is_valid());
    assert!(!Civil { second: 60, ..base }.is_valid());
}

#[test]
fn file_age_floors_fractional_seconds_and_saturates() {
    assert_eq!(
        FileTime {
            secs: 100,
            nanos: 999_999_999
        }
        .age_secs(UtcInstant { secs: 130 }),
        29
    );
    assert_eq!(
        FileTime {
            secs: 100,
            nanos: 0
        }
        .age_secs(UtcInstant { secs: 130 }),
        30
    );
    assert_eq!(
        FileTime {
            secs: 130,
            nanos: 1
        }
        .age_secs(UtcInstant { secs: 100 }),
        -31
    );
    assert_eq!(
        FileTime { secs: 0, nanos: 1 }.age_secs(UtcInstant { secs: 1 }),
        0
    );
    assert_eq!(
        FileTime {
            secs: i64::MIN,
            nanos: 1
        }
        .age_secs(UtcInstant { secs: i64::MAX }),
        i64::MAX
    );
    assert_eq!(
        FileTime {
            secs: i64::MAX,
            nanos: 0
        }
        .age_secs(UtcInstant { secs: i64::MIN }),
        i64::MIN
    );
}
