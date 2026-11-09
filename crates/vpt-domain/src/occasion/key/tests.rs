use super::*;
use crate::digest::Sha256Digest;
use crate::occasion::OccasionId;

#[test]
fn manual_identity_uses_full_sanitized_title_and_exact_start() {
    let at = OccasionTime::parse("2026-09-30T23:00:00.123456789012Z").unwrap();
    let key = OccasionKey::manual(&at, "../Private: STRAẞE / sync!").unwrap();
    assert_eq!(
        key,
        OccasionKey::manual(&at, "private-strasse-sync").unwrap()
    );
    assert_ne!(
        key,
        OccasionKey::manual(&at, "private-strasse-sync-extra").unwrap()
    );
    let other = OccasionTime::parse("2026-09-30T23:00:00.123456789013Z").unwrap();
    assert_ne!(
        key,
        OccasionKey::manual(&other, "private-strasse-sync").unwrap()
    );
    assert_eq!(key.source(), OccasionSource::Manual);
    assert_eq!(OccasionKey::parse(&key.encoded()).unwrap(), key);
    assert!(!format!("{key:?}").contains("private"));
}

#[test]
fn provider_keys_frame_fields_and_preserve_provider_case() {
    let keys = [
        OccasionKey::dam("Private:object").unwrap(),
        OccasionKey::google("Private:cal", "event").unwrap(),
        OccasionKey::google("Private", "cal:event").unwrap(),
        OccasionKey::google("private:cal", "event").unwrap(),
    ];
    for (index, key) in keys.iter().enumerate() {
        assert_eq!(OccasionKey::parse(&key.encoded()).unwrap(), *key);
        for other in &keys[index + 1..] {
            assert_ne!(key.encoded(), other.encoded());
        }
        assert!(!format!("{key:?}").contains("Private"));
    }
    assert!(OccasionKey::dam("").is_err());
    assert!(OccasionKey::google("", "event").is_err());
    assert!(OccasionKey::dam("bad\0id").is_err());
    assert!(OccasionKey::dam(&"x".repeat(65_537)).is_err());
    for bad in [
        "dam:01:a",
        "dam:2:a",
        "dam:1:aextra",
        "google:1:a",
        "manual:1:a1:b",
        "other:1:a",
    ] {
        assert!(OccasionKey::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn occasion_ids_use_occasion_local_time_and_a_separate_identity_type() {
    for text in ["2026-09-30T23:30:00-06:00", "1990-12-31T15:59:60-08:00"] {
        let at = OccasionTime::parse(text).unwrap();
        let id = OccasionId::derive(&at, &Sha256Digest([0xab; 32]));
        assert_eq!(
            id.as_str(),
            format!("{}-abababababab", at.local_timestamp())
        );
        assert_eq!(id.date(), at.date());
        assert_eq!(OccasionId::parse(id.as_str()).unwrap(), id);
    }
    for value in [
        "2026-09-31T000000-abcdefabcdef",
        "2026-09-30T240000-abcdefabcdef",
        "2026-09-30T000000-ABCDEFABCDEF",
        "é",
    ] {
        assert!(OccasionId::parse(value).is_err());
    }
}
