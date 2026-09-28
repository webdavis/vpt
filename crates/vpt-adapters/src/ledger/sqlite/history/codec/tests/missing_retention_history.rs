use super::*;
use serde_json::json;
use vpt_application::ports::LedgerError;

#[test]
fn missing_retention_history_round_trips_positive_full_width_ids() {
    for intent_id in [1, 71, u64::MAX] {
        let fact = HistoryFact::MissingRetentionHistory { intent_id };
        let encoded = encode(&fact).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap(),
            json!({"tag":"missing_retention_history","intent_id":intent_id})
        );
        assert_eq!(decode(&encoded).unwrap(), fact);
    }
}

#[test]
fn missing_retention_history_decoder_checks_id_and_closed_tag() {
    let valid = r#"{"tag":"missing_retention_history","intent_id":71,"later":"ignored"}"#;
    assert_eq!(
        decode(valid).unwrap(),
        HistoryFact::MissingRetentionHistory { intent_id: 71 }
    );
    for invalid in [
        r#"{"tag":"missing_retention_history"}"#,
        r#"{"tag":"missing_retention_history","intent_id":0}"#,
        r#"{"tag":"missing_retention_history","intent_id":-1}"#,
        r#"{"tag":"missing_retention_history","intent_id":"1"}"#,
        r#"{"tag":"missing_retention_history","intent_id":18446744073709551616}"#,
        r#"{"tag":"missing_retention_history","intent_id":true}"#,
        r#"{"tag":"future_retention_gap","intent_id":71}"#,
    ] {
        assert!(
            matches!(decode(invalid), Err(LedgerError::Corrupt(_))),
            "{invalid}"
        );
    }
}

#[test]
fn missing_retention_history_encoder_refuses_zero() {
    assert!(matches!(
        encode(&HistoryFact::MissingRetentionHistory { intent_id: 0 }),
        Err(LedgerError::Corrupt(_))
    ));
}
