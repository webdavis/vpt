use super::*;

#[test]
fn word_array_accepts_the_exact_limit_and_refuses_one_more() {
    let mut value = document("mlx");
    let words = vec![json!({"word":"hello","start":0.0,"end":0.5}); 65_536];
    value["transcription"]["en"]["chunks"][0]["words"] = json!(words);
    assert!(decode_value(&value, "mlx").is_ok());
    value["transcription"]["en"]["chunks"][0]["words"]
        .as_array_mut()
        .unwrap()
        .push(json!(null));
    assert!(matches!(
        decode_value(&value, "mlx"),
        Err(EngineDecodeError::Limit(crate::LimitViolation::Array(
            65_537
        )))
    ));
}

#[test]
fn a_similarly_named_additive_array_keeps_the_small_array_limit() {
    let mut value = document("mlx");
    value["nested"] = json!({"transcription":{"en":{"chunks":vec![0;257]}}});
    assert!(matches!(
        decode_value(&value, "mlx"),
        Err(EngineDecodeError::Limit(crate::LimitViolation::Array(257)))
    ));
}
