mod archived_offset;

use super::*;

fn digest() -> Sha256Digest {
    let mut bytes = [0u8; 32];
    bytes[..6].copy_from_slice(&[0x4f, 0x3a, 0xb1, 0x9c, 0x02, 0xde]);
    Sha256Digest(bytes)
}

#[test]
fn derived_identity_uses_local_capture_time_and_digest_prefix() {
    let id = RecordingId::derive(
        UtcInstant {
            secs: 1_787_604_456,
        },
        UtcOffset { secs: -21_600 },
        &digest(),
    )
    .expect("representable");
    assert_eq!(id.as_str(), "2026-08-24T144736-4f3ab19c02de");
    assert_eq!(id.local_timestamp(), "2026-08-24T144736");
    assert_eq!(id.capture_date(), "2026-08-24");
    assert_eq!(id.hash12(), "4f3ab19c02de");
    assert_eq!(id.to_string(), id.as_str());
}

#[test]
fn local_timestamp_uses_compact_clock_and_applies_offset() {
    assert_eq!(
        local_timestamp(UtcInstant { secs: 0 }, UtcOffset { secs: 0 }),
        "1970-01-01T000000"
    );
    assert_eq!(
        local_timestamp(UtcInstant { secs: 0 }, UtcOffset { secs: -3_600 }),
        "1969-12-31T230000"
    );
}

#[test]
fn valid_identity_and_timestamp_parse_back_to_civil_time() {
    let text = "2026-08-24T144736-4f3ab19c02de";
    assert_eq!(RecordingId::parse(text).expect("valid").as_str(), text);
    let civil = parse_local_timestamp("2026-08-24T144736").expect("civil");
    assert_eq!((civil.year, civil.month, civil.day), (2026, 8, 24));
    assert_eq!((civil.hour, civil.minute, civil.second), (14, 47, 36));
    assert_eq!(
        civil.instant(UtcOffset { secs: -21_600 }),
        Some(UtcInstant {
            secs: 1_787_604_456
        })
    );
}

#[test]
fn identity_requires_exact_ascii_byte_shape() {
    for text in [
        "2026-08-24-4f3ab19c02de",
        "2026-08-24T144736-4f3ab19c02d",
        "2026-08-24T144736-4f3ab19c02dee",
        "2026-08-24T144736_4f3ab19c02de",
        "2026-08-24T144736-4f3ab19c02dg",
        "2026-08-24T144736-4F3AB19C02DE",
        "2026-08-24T+14736-4f3ab19c02de",
        "../2026-08-24T144736-4f3ab19c02de",
        "2026-08-24T1447é-4f3ab19c02de",
        "2026-08-24T144736-4f3ab19c02dé",
    ] {
        assert_eq!(
            RecordingId::parse(text),
            Err(IdentityError::Malformed),
            "{text}"
        );
    }
}

#[test]
fn timestamp_requires_exact_ascii_byte_shape() {
    for text in [
        "2026-08-24T14:47:36",
        "2026-08-24T144736-4f3ab19c02de",
        "2026/08/24T144736",
        "2026-08-24t144736",
        "2026-08-24T+14736",
        "2026-08-24T1447é",
        "2026-08-24T14473é",
    ] {
        assert_eq!(
            parse_local_timestamp(text),
            Err(IdentityError::Malformed),
            "{text}"
        );
    }
}

#[test]
fn calendar_and_clock_bounds_apply_to_both_parsers() {
    for timestamp in [
        "2023-02-29T144736",
        "1900-02-29T144736",
        "2026-00-01T000000",
        "2026-13-01T000000",
        "2026-04-31T000000",
        "2026-08-00T000000",
        "2026-08-24T240000",
        "2026-08-24T146000",
        "2026-08-24T144760",
    ] {
        assert_eq!(
            parse_local_timestamp(timestamp),
            Err(IdentityError::Malformed)
        );
        assert_eq!(
            RecordingId::parse(&format!("{timestamp}-4f3ab19c02de")),
            Err(IdentityError::Malformed)
        );
    }
    for timestamp in [
        "2024-02-29T235959",
        "2000-02-29T000000",
        "0000-01-01T000000",
    ] {
        assert!(parse_local_timestamp(timestamp).is_ok(), "{timestamp}");
        assert!(RecordingId::parse(&format!("{timestamp}-4f3ab19c02de")).is_ok());
    }
}

#[test]
fn derive_checks_four_digit_local_year_after_offset() {
    let year_ten_thousand = UtcInstant {
        secs: 253_402_300_800,
    };
    assert_eq!(
        RecordingId::derive(year_ten_thousand, UtcOffset { secs: 0 }, &digest()),
        Err(IdentityError::Unrepresentable)
    );
    assert_eq!(
        RecordingId::derive(year_ten_thousand, UtcOffset { secs: -3_600 }, &digest())
            .expect("local year is 9999")
            .capture_date(),
        "9999-12-31"
    );
    let year_zero = UtcInstant {
        secs: -62_167_219_200,
    };
    assert_eq!(
        RecordingId::derive(year_zero, UtcOffset { secs: 0 }, &digest())
            .expect("four-digit year zero")
            .capture_date(),
        "0000-01-01"
    );
    assert_eq!(
        RecordingId::derive(year_zero, UtcOffset { secs: -1 }, &digest()),
        Err(IdentityError::Unrepresentable)
    );
    assert_eq!(
        RecordingId::derive(
            UtcInstant { secs: i64::MAX },
            UtcOffset { secs: 0 },
            &digest()
        ),
        Err(IdentityError::Unrepresentable)
    );
}
