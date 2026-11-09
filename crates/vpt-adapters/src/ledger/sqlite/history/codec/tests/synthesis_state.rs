use super::*;
use vpt_domain::stage::StageState;

#[test]
fn synthesis_transitions_round_trip_and_unknown_states_are_private_corruption() {
    let fact = HistoryFact::SynthesisStateChanged {
        recording: crate::ledger::contract::record(13).id,
        from: StageState::Disabled,
        to: StageState::Pending,
    };
    let stored = encode(&fact).unwrap();
    assert_eq!(decode(&stored).unwrap(), fact);
    let base: serde_json::Value = serde_json::from_str(&stored).unwrap();
    for field in ["from", "to"] {
        for value in [
            serde_json::json!("private invalid state"),
            serde_json::Value::Null,
            serde_json::json!(1),
        ] {
            let mut invalid = base.clone();
            invalid[field] = value;
            let error = decode(&invalid.to_string()).unwrap_err();
            assert!(!format!("{error:?}").contains("private invalid state"));
        }
    }
}
