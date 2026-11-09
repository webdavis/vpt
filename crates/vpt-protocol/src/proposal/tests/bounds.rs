use super::*;

#[test]
fn proposal_bytes_and_unicode_text_are_bounded_at_the_specified_limits() {
    let mut bytes = serde_json::to_vec(&document()).unwrap();
    bytes.resize(1_048_576, b' ');
    assert!(decode(&bytes).is_ok());
    bytes.push(b' ');
    assert_eq!(
        decode(&bytes),
        Err(ProposalDecodeError::Limit(LimitViolation::Bytes(1_048_577)))
    );
    let mut doc = document();
    doc["summary"][0]["text"] = json!("λ".repeat(65_536));
    assert!(decoded(&doc).is_ok());
    doc["summary"][0]["text"] = json!("λ".repeat(65_537));
    assert_eq!(
        decoded(&doc),
        Err(ProposalDecodeError::Limit(LimitViolation::Text(65_537)))
    );
}

#[test]
fn only_top_level_summary_and_actions_use_the_larger_array_limit() {
    for field in ["summary", "actions"] {
        let mut doc = document();
        doc[field] = json!(vec![json!({"text":"claim", "sources":[]}); 300]);
        assert!(decoded(&doc).is_ok());
        doc[field] = json!(vec![Value::Null; 65_536]);
        assert_eq!(decoded(&doc), Err(ProposalDecodeError::Invalid("document")));
        doc[field] = json!(vec![Value::Null; 65_537]);
        assert_eq!(
            decoded(&doc),
            Err(ProposalDecodeError::Limit(LimitViolation::Array(65_537)))
        );
    }
    for field in ["tags", "relations", "segments", "future"] {
        let mut doc = document();
        doc[field] = json!(vec![Value::Null; 257]);
        assert_eq!(
            decoded(&doc),
            Err(ProposalDecodeError::Limit(LimitViolation::Array(257)))
        );
    }
    let mut doc = document();
    doc["future"] = json!({"summary":vec![Value::Null;257]});
    assert_eq!(
        decoded(&doc),
        Err(ProposalDecodeError::Limit(LimitViolation::Array(257)))
    );
    let bytes = serde_json::to_vec(&json!({"summary":vec![Value::Null;257]})).unwrap();
    assert_eq!(
        crate::read_bounded(&bytes, &crate::Limits::incoming()),
        Err(LimitViolation::Array(257))
    );
}

#[test]
fn claim_sources_keep_the_general_array_limit_and_unknown_data_keeps_the_depth_limit() {
    let mut doc = document();
    doc["summary"][0]["sources"] = json!(vec!["00:00-00:01"; 256]);
    assert!(decoded(&doc).is_ok());
    doc["summary"][0]["sources"] = json!(vec!["00:00-00:01"; 257]);
    assert_eq!(
        decoded(&doc),
        Err(ProposalDecodeError::Limit(LimitViolation::Array(257)))
    );
    let mut doc = document();
    let nested = (0..8).fold(json!("Private"), |value, _| json!([value]));
    doc["future"] = nested;
    assert_eq!(
        decoded(&doc),
        Err(ProposalDecodeError::Limit(LimitViolation::Depth(9)))
    );
}
