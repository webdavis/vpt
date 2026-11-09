use super::*;

#[test]
fn known_engine_arrays_allow_more_than_the_general_array_limit() {
    let mut value = document();
    let word = json!({"text": "word", "start": 0.0, "end": 0.0});
    value["segments"][0]["words"] = json!(vec![word; 257]);
    assert_eq!(
        decoded(&value).unwrap().document.segments[0].words.len(),
        257
    );
    let segment = document()["segments"][0].clone();
    value["segments"] = json!(vec![segment; 257]);
    assert_eq!(decoded(&value).unwrap().document.segments.len(), 257);
}

#[test]
fn the_large_array_limit_is_checked_before_decoding_the_overflow_element() {
    for (prefix, suffix) in [
        ("{\"segments\":[", "]}"),
        ("{\"segments\":[{\"words\":[", "]}]}"),
    ] {
        let input = format!("{prefix}{}broken{suffix}", "null,".repeat(65_536));
        assert_eq!(
            decode(input.as_bytes(), &expected(), 5.0),
            Err(EngineDecodeError::Limit(LimitViolation::Array(65_537)))
        );
    }
}

#[test]
fn additive_arrays_and_nested_names_keep_the_general_limit() {
    for pointer in ["/future", "/engine/future"] {
        let mut value = document();
        if pointer == "/future" {
            value["future"] = json!(vec![0; 257]);
        } else {
            value["engine"]["future"] = json!(vec![0; 257]);
        }
        assert_eq!(
            decoded(&value),
            Err(EngineDecodeError::Limit(LimitViolation::Array(257)))
        );
    }
    let mut value = document();
    value["future"] = json!({"segments": vec![0; 257]});
    assert_eq!(
        decoded(&value),
        Err(EngineDecodeError::Limit(LimitViolation::Array(257)))
    );
}

#[test]
fn engine_byte_budget_is_sixteen_mebibytes() {
    let mut bytes = serde_json::to_vec(&document()).unwrap();
    bytes.resize(16_777_216, b' ');
    assert!(decode(&bytes, &expected(), 5.0).is_ok());
    bytes.push(b' ');
    assert_eq!(
        decode(&bytes, &expected(), 5.0),
        Err(EngineDecodeError::Limit(LimitViolation::Bytes(16_777_217)))
    );
}

#[test]
fn engine_text_and_depth_are_bounded_and_malformed_output_is_safe() {
    let mut value = document();
    value["segments"][0]["words"][0]["text"] = json!("a".repeat(65_537));
    assert_eq!(
        decoded(&value),
        Err(EngineDecodeError::Limit(LimitViolation::Text(65_537)))
    );
    let bytes = br#"{"private-canary": [[[[[[[[0]]]]]]]]}"#;
    assert_eq!(
        decode(bytes, &expected(), 5.0),
        Err(EngineDecodeError::Limit(LimitViolation::Depth(9)))
    );
    let error = decode(b"private-canary", &expected(), 5.0).unwrap_err();
    assert!(!format!("{error:?}").contains("private-canary"));
}
