use super::*;
use vpt_domain::uncertainty::Class;

fn id() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}
fn flag() -> LexicalFlag {
    LexicalFlag {
        class: Class::AgreedUnverified,
        ranges: vec![2..3, 6..7],
        record_text: "Alice".into(),
        alternative_text: None,
        confidence: Some(0.9),
    }
}

#[test]
fn every_identity_input_changes_the_flag_id() {
    let original = flag();
    let digest = lexical_flag_id(&id(), &original);
    assert_eq!(
        digest.hex(),
        "4b7b401b1768182e57e6406ddad5751f3af2ade7ba4c3ed64c312717af41da95"
    );
    let mut changed = original.clone();
    changed.class = Class::ProperNoun;
    assert_ne!(digest, lexical_flag_id(&id(), &changed));
    changed = original.clone();
    changed.ranges[1] = 7..8;
    assert_ne!(digest, lexical_flag_id(&id(), &changed));
    changed = original.clone();
    changed.record_text = "ALICE".into();
    assert_ne!(digest, lexical_flag_id(&id(), &changed));
    changed = original.clone();
    changed.alternative_text = Some("Alice".into());
    assert_ne!(digest, lexical_flag_id(&id(), &changed));
    assert_ne!(
        digest,
        lexical_flag_id(
            &RecordingId::parse("2026-08-24T144737-4f3ab19c02de").unwrap(),
            &original
        )
    );
}

#[test]
fn confidence_updates_preserve_the_identity_and_hash_framing_is_unambiguous() {
    let mut changed = flag();
    changed.confidence = None;
    assert_eq!(
        lexical_flag_id(&id(), &flag()),
        lexical_flag_id(&id(), &changed)
    );
    changed.alternative_text = Some(String::new());
    assert_ne!(
        lexical_flag_id(&id(), &flag()),
        lexical_flag_id(&id(), &changed)
    );
    let mut first = flag();
    first.record_text = "ab".into();
    first.alternative_text = Some("c".into());
    let mut second = first.clone();
    second.record_text = "a".into();
    second.alternative_text = Some("bc".into());
    assert_ne!(
        lexical_flag_id(&id(), &first),
        lexical_flag_id(&id(), &second)
    );
}

#[test]
fn diagnostics_have_separate_ids_by_record_and_kind() {
    let single = diagnostic_flag_id(&id(), Diagnostic::SingleEngine);
    assert_ne!(single, diagnostic_flag_id(&id(), Diagnostic::Divergent));
    assert_ne!(
        single,
        diagnostic_flag_id(&id(), Diagnostic::LanguageMismatch)
    );
    assert_ne!(single, lexical_flag_id(&id(), &flag()));
    assert_ne!(
        single,
        diagnostic_flag_id(
            &RecordingId::parse("2026-08-24T144737-4f3ab19c02de").unwrap(),
            Diagnostic::SingleEngine
        )
    );
}

#[test]
fn verification_identity_tracks_recording_artifact_claim_position_and_digest() {
    use vpt_domain::{grounding::Finding, note::NoteStage};
    let original = VerificationFlag {
        artifact: NoteStage::Analysis,
        claim_index: 3,
        claim_digest: Sha256Digest(Sha256::digest(b"private claim").into()),
        record_text: "private claim".into(),
        finding: Finding::Unsourced,
        sources: vec![],
    };
    let digest = verification_flag_id(&id(), &original);
    for variant in 0..3 {
        let mut changed = original.clone();
        match variant {
            0 => changed.artifact = NoteStage::Transcript,
            1 => changed.claim_index += 1,
            _ => changed.claim_digest.0[0] ^= 1,
        }
        assert_ne!(verification_flag_id(&id(), &changed), digest);
    }
    let other = RecordingId::parse("2026-08-24T144737-4f3ab19c02de").unwrap();
    assert_ne!(verification_flag_id(&other, &original), digest);
    let mut reclassified = original.clone();
    reclassified.finding = Finding::BadReference;
    assert_eq!(verification_flag_id(&id(), &reclassified), digest);
    assert_ne!(digest, lexical_flag_id(&id(), &flag()));
    assert_ne!(digest, diagnostic_flag_id(&id(), Diagnostic::SingleEngine));
}
