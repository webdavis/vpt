use super::*;
use serde_json::{Value, json};

fn expected() -> ExpectedEngine {
    ExpectedEngine {
        name: "apple".into(),
        family: "apple".into(),
        local: true,
    }
}

fn document() -> Value {
    json!({
        "schema": "vpt.engine/1",
        "engine": {"name": "apple", "tool": "vpt-macos", "version": "1.0.0", "model": "SpeechTranscriber", "family": "apple"},
        "language": "en-US", "local": true,
        "segments": [{"start": 0.0, "end": 4.2, "text": "Hello world.", "words": [
            {"text": "Hello", "start": 0.0, "end": 0.5, "confidence": 0.97},
            {"text": "world", "start": 0.5, "end": 1.0}
        ]}]
    })
}

fn decoded(value: &Value) -> Result<DecodedEngine, EngineDecodeError> {
    decode(&serde_json::to_vec(value).unwrap(), &expected(), 5.0)
}

#[test]
fn accepts_timed_words_and_preserves_absent_confidence() {
    let result = decoded(&document()).unwrap();
    assert_eq!(result.document.segments[0].words[0].confidence, Some(0.97));
    assert_eq!(result.document.segments[0].words[1].confidence, None);
    assert_eq!(result.document.language, "en-US");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn unknown_major_version_is_named_without_echoing_document_content() {
    let mut value = document();
    value["schema"] = json!("vpt.engine/2");
    assert_eq!(
        decoded(&value),
        Err(EngineDecodeError::UnsupportedVersion(2))
    );
}

#[test]
fn invalid_identity_empty_transcripts_and_ranges_are_refused() {
    for (pointer, replacement, category) in [
        ("/engine/name", json!("another"), "engine identity"),
        ("/engine/family", json!("whisper"), "engine identity"),
        ("/local", json!(false), "engine identity"),
        ("/engine/tool", json!(""), "engine metadata"),
        ("/engine/version", json!(" "), "engine metadata"),
        ("/engine/model", json!(""), "engine metadata"),
        ("/language", json!(""), "language"),
        ("/segments", json!([]), "empty transcript"),
        ("/segments/0/words", json!([]), "empty transcript"),
        ("/segments/0/text", json!(" "), "empty transcript"),
        ("/segments/0/words/0/text", json!(" "), "word text"),
        ("/segments/0/start", json!(-1), "segment range"),
        ("/segments/0/end", json!(6), "segment range"),
        ("/segments/0/words/0/end", json!(4.3), "word range"),
        ("/segments/0/words/0/start", json!(0.6), "word range"),
        ("/segments/0/words/1/start", json!(-1), "word range"),
        ("/segments/0/words/0/confidence", json!(1.1), "confidence"),
        ("/segments/0/words/0/confidence", json!(-0.1), "confidence"),
    ] {
        let mut value = document();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_eq!(
            decoded(&value),
            Err(EngineDecodeError::Invalid(category)),
            "{pointer}"
        );
    }
}

#[test]
fn recording_duration_and_word_order_are_validated() {
    for duration in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            decode(
                &serde_json::to_vec(&document()).unwrap(),
                &expected(),
                duration
            ),
            Err(EngineDecodeError::Invalid("duration"))
        );
    }
    let mut value = document();
    value["segments"][0]["words"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        decoded(&value),
        Err(EngineDecodeError::Invalid("word order"))
    );
    let mut value = document();
    let mut later = value["segments"][0].clone();
    later["start"] = json!(1.5);
    later["words"] = json!([]);
    value["segments"].as_array_mut().unwrap().insert(0, later);
    assert_eq!(
        decoded(&value),
        Err(EngineDecodeError::Invalid("segment order"))
    );
}

#[test]
fn additive_fields_are_accepted_with_named_diagnostics() {
    let mut value = document();
    value["future"] = json!(true);
    value["engine"]["revision"] = json!("canary");
    value["segments"][0]["speaker"] = json!("canary");
    value["segments"][0]["words"][0]["score_kind"] = json!("canary");
    let result = decoded(&value).unwrap();
    for name in [
        "future",
        "engine.revision",
        "segments.speaker",
        "segments.words.score_kind",
    ] {
        assert!(
            result
                .diagnostics
                .iter()
                .any(|detail| detail.contains(name)),
            "{name}"
        );
    }
    assert!(!result.diagnostics.join(" ").contains("canary"));
}

mod limits;

#[test]
fn an_explicit_confidence_requires_a_number() {
    let mut value = document();
    value["segments"][0]["words"][0]["confidence"] = Value::Null;
    assert_eq!(decoded(&value), Err(EngineDecodeError::Invalid("document")));
}

#[test]
fn zero_length_ranges_and_confidence_endpoints_are_valid() {
    let mut value = document();
    value["segments"][0]["start"] = json!(0);
    value["segments"][0]["end"] = json!(0);
    for (index, word) in value["segments"][0]["words"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        word["start"] = json!(0);
        word["end"] = json!(0);
        word["confidence"] = json!(index);
    }
    assert!(decode(&serde_json::to_vec(&value).unwrap(), &expected(), 0.0).is_ok());
}
