use super::*;
mod limits;
use serde_json::{Value, json};

#[test]
fn normalized_document_round_trips_through_the_public_engine_contract() {
    let decoded = decode_value(&document("mlx"), "mlx").unwrap();
    let bytes = serde_json::to_vec(&decoded.document).unwrap();
    assert!(!std::str::from_utf8(&bytes).unwrap().contains("confidence"));
    let expected = crate::engine::ExpectedEngine {
        name: "local-whisper".into(),
        family: "whisper".into(),
        local: true,
    };
    let reloaded = crate::engine::decode(&bytes, &expected, 1.5).unwrap();
    assert_eq!(decoded.document, reloaded.document);
}

fn request(device: &str) -> WhisplyRequest<'_> {
    WhisplyRequest {
        name: "local-whisper",
        language: "en-US",
        backend_language: "en",
        device,
        duration: 1.5,
    }
}

fn document(device: &str) -> Value {
    json!({"id":"file_001", "created":"2026-09-28 12:00:00",
        "input_filepath":"/private-canary/audio.m4a", "output_filepath":"/private-canary/output",
        "written_files":["/private-canary/output.json"], "device":device, "model":"large-v3-turbo",
        "transcription":{"en":{"text":"Hello world", "chunks":[{"timestamp":[0.0,1.5],"text":"Hello world",
            "words":[{"word":"Hello", "start":0.0,"end":0.5}, {"word":"world", "start":0.5,"end":1.5}]}]}}})
}

fn decode_value(value: &Value, device: &str) -> Result<DecodedEngine, EngineDecodeError> {
    decode(&serde_json::to_vec(value).unwrap(), &request(device))
}

#[test]
fn mlx_document_preserves_words_timing_model_and_absent_confidence() {
    let result = decode_value(&document("mlx"), "mlx").unwrap();
    assert_eq!(result.document.engine.name, "local-whisper");
    assert_eq!(result.document.engine.family, "whisper");
    assert_eq!(result.document.engine.tool, "whisply");
    assert_eq!(result.document.engine.model, "large-v3-turbo");
    assert_eq!(result.document.engine.version, "unreported");
    assert_eq!(result.document.language, "en-US");
    assert!(result.document.local);
    assert_eq!(result.document.segments[0].words[1].end, 1.5);
    assert_eq!(result.document.segments[0].words[0].confidence, None);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|detail| detail.contains("language") && detail.contains("requested"))
    );
    assert!(!format!("{:?}", result.diagnostics).contains("private-canary"));
}

#[test]
fn supplied_cpu_scores_are_confidence_and_missing_scores_stay_absent() {
    let mut value = document("cpu");
    value["transcription"]["en"]["chunks"][0]["words"][0]["score"] = json!(0.81);
    let result = decode_value(&value, "cpu").unwrap();
    assert_eq!(result.document.segments[0].words[0].confidence, Some(0.81));
    assert_eq!(result.document.segments[0].words[1].confidence, None);
}

#[test]
fn unknown_fields_are_named_without_exposing_their_values() {
    let mut value = document("mlx");
    value["future"] = json!("private-canary");
    value["transcription"]["en"]["chunks"][0]["future_chunk"] = json!("private-canary");
    let result = decode_value(&value, "mlx").unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|detail| detail.contains("future_chunk"))
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|detail| detail.contains("future"))
    );
    assert!(!format!("{:?}", result.diagnostics).contains("private-canary"));
}

#[test]
fn missing_requested_language_and_changed_device_are_refused() {
    let mut value = document("mlx");
    value["transcription"]["fr"] = value["transcription"]["en"].take();
    assert!(decode_value(&value, "mlx").is_err());
    assert!(decode_value(&document("cpu"), "mlx").is_err());
}

#[test]
fn backend_device_resolution_accepts_auto_and_documented_gpu_mapping() {
    for (configured, actual) in [
        ("auto", "cpu"),
        ("auto", "mlx"),
        ("auto", "cuda:0"),
        ("gpu", "cuda:0"),
        ("gpu", "cpu"),
    ] {
        let value = document(actual);
        assert!(
            decode_value(&value, configured).is_ok(),
            "{configured} -> {actual}"
        );
    }
    assert!(decode_value(&document("private-canary"), "auto").is_err());
}

#[test]
fn invalid_timing_empty_words_and_invalid_scores_are_engine_failures() {
    for replacement in [
        json!(-0.01),
        json!(1.01),
        json!(null),
        json!("private-canary"),
    ] {
        let mut value = document("cpu");
        value["transcription"]["en"]["chunks"][0]["words"][0]["score"] = replacement;
        assert!(decode_value(&value, "cpu").is_err());
    }
    for replacement in [
        json!([1.5, 0.0]),
        json!([0.0, 1.51]),
        json!([0.0]),
        json!([0.0, 1.0, 1.5]),
    ] {
        let mut value = document("mlx");
        value["transcription"]["en"]["chunks"][0]["timestamp"] = replacement;
        assert!(decode_value(&value, "mlx").is_err());
    }
    let mut value = document("mlx");
    value["transcription"]["en"]["chunks"][0]["words"] = json!([]);
    assert!(decode_value(&value, "mlx").is_err());
}

#[test]
fn larger_word_arrays_are_permitted_only_at_the_backend_word_location() {
    let mut value = document("mlx");
    let word = json!({"word":"hello", "start":0.0,"end":0.5});
    value["transcription"]["en"]["chunks"][0]["words"] = json!(vec![word; 257]);
    assert!(decode_value(&value, "mlx").is_ok());
    value["extra"] = json!(vec![0; 257]);
    assert!(matches!(
        decode_value(&value, "mlx"),
        Err(EngineDecodeError::Limit(crate::LimitViolation::Array(257)))
    ));
}

#[test]
fn excessive_input_is_rejected_before_json_parsing() {
    assert!(matches!(
        decode(&vec![b' '; 16_777_217], &request("mlx")),
        Err(EngineDecodeError::Limit(crate::LimitViolation::Bytes(
            16_777_217
        )))
    ));
}
