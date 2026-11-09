use super::*;
use crate::ledger::contract::transcript::fixture;

#[test]
fn checker_metadata_survives_serialization_and_legacy_bodies_remain_readable() {
    let mut accepted = fixture();
    accepted.from_checker = false;
    let mut checker = accepted.transcript.engine.clone();
    checker.name = "comparison".into();
    accepted.checker_engine = Some(checker);
    let encoded = encode(&accepted).unwrap();
    let read = |body: &str| {
        decode(
            accepted.recording.clone(),
            &accepted.transcript.engine.name,
            body,
            accepted.accepted_at,
        )
        .unwrap()
    };
    assert_eq!(read(&encoded), accepted);
    let mut legacy: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    legacy.as_object_mut().unwrap().remove("checker_engine");
    let read = read(&serde_json::to_string(&legacy).unwrap());
    accepted.checker_engine = None;
    assert_eq!(read, accepted);
}

#[test]
fn invalid_checker_metadata_is_refused_on_both_write_and_read() {
    for variant in 0..5 {
        let mut accepted = fixture();
        accepted.from_checker = false;
        let mut checker = accepted.transcript.engine.clone();
        checker.name = "comparison".into();
        match variant {
            0 => accepted.from_checker = true,
            1 => checker.name = accepted.transcript.engine.name.clone(),
            2 => checker.model = " ".into(),
            3 => checker.tool = "x".repeat(65_537),
            _ => checker.version = "🦆".repeat(65_537),
        }
        let mut raw: serde_json::Value = serde_json::from_str(&encode(&accepted).unwrap()).unwrap();
        raw["checker_engine"] =
            serde_json::to_value(crate::transcript_document::to_metadata(&checker)).unwrap();
        accepted.checker_engine = Some(checker);
        assert_eq!(encode(&accepted).unwrap_err(), corrupt());
        assert_eq!(
            decode(
                accepted.recording.clone(),
                &accepted.transcript.engine.name,
                &serde_json::to_string(&raw).unwrap(),
                accepted.accepted_at
            )
            .unwrap_err(),
            corrupt()
        );
    }
}
