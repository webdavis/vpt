use super::ArtifactKind;

#[test]
fn artifact_kind_spelling_is_exact_and_note_stages_are_distinct() {
    assert_eq!(ArtifactKind::Audio.as_str(), "audio");
    assert_eq!(ArtifactKind::parse("audio"), Some(ArtifactKind::Audio));
    assert_eq!(
        ArtifactKind::parse("transcript"),
        Some(ArtifactKind::Transcript)
    );
    assert_eq!(
        ArtifactKind::parse("analysis"),
        Some(ArtifactKind::Analysis)
    );
    assert_eq!(ArtifactKind::parse("Audio"), None);
    assert_eq!(ArtifactKind::parse("note"), None);
}
