use super::*;
use vpt_application::ports::DirtyPublication;

#[test]
fn saved_proposal_vocabulary_preserves_exact_publication_and_reads_legacy_records() {
    let mut accepted = accepted();
    let mut legacy: serde_json::Value = serde_json::from_str(&encode(&accepted).unwrap()).unwrap();
    legacy["version"] = 1.into();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("vocabulary_publication");
    assert_eq!(
        decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &legacy.to_string(),
            accepted.accepted_at
        )
        .unwrap(),
        accepted
    );
    accepted.vocabulary_publication = Some(DirtyPublication {
        target: "/private/vocabulary/known-tags.txt".into(),
        expected_previous: Some(Sha256Digest([4; 32])),
        intended: Sha256Digest([5; 32]),
        recorded_at: accepted.accepted_at,
    });
    let body = encode(&accepted).unwrap();
    assert_eq!(
        decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &body,
            accepted.accepted_at
        )
        .unwrap(),
        accepted
    );
}

#[test]
fn corrupt_proposal_publication_metadata_is_refused_without_private_values() {
    let accepted = accepted();
    let original: serde_json::Value = serde_json::from_str(&encode(&accepted).unwrap()).unwrap();
    for variant in 0..8 {
        let mut value = original.clone();
        value["vocabulary_publication"] = serde_json::json!({
            "target":"/private/known-tags.txt", "expected_previous":null,
            "intended":Sha256Digest([5;32]).hex(),
        });
        let entry = &mut value["vocabulary_publication"];
        match variant {
            0 => entry["target"] = "private-relative".into(),
            1 => entry["target"] = "/private/../known-tags.txt".into(),
            2 => entry["target"] = "/private/\nknown-tags.txt".into(),
            3 => entry["intended"] = "private-invalid-digest".into(),
            4 => entry["expected_previous"] = entry["intended"].clone(),
            5 => entry["target"] = format!("/{}", "x".repeat(65_536)).into(),
            6 => entry["unknown"] = "private-value".into(),
            _ => value["version"] = 1.into(),
        }
        let result = decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &value.to_string(),
            accepted.accepted_at,
        );
        assert_eq!(result, Err(invalid()), "variant {variant}");
        assert!(!format!("{result:?}").contains("private"));
    }
}
