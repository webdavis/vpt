use super::*;
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    time::{UtcInstant, UtcOffset},
};
fn document<'a>(id: &'a RecordingId, body: &'a str) -> Document<'a> {
    Document {
        kind: Kind::Released,
        identity: Identity::Recording(id),
        note: "released/copy.md",
        stem: "copy",
        digest: Sha256Digest([3; 32]),
        body,
        provenance: Provenance {
            captured: Some((UtcInstant { secs: 0 }, UtcOffset { secs: 0 })),
            occasion_at: None,
            engines: &[],
            open_flags: 2,
            reviewed: false,
        },
        generated_at: UtcInstant { secs: 1 },
    }
}
#[test]
fn handoff_preserves_body_and_only_generation_time_changes_between_invocations() {
    let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let body = "\r\n# Content\r\n\nKeep *these* bytes.\n";
    let mut doc = document(&id, body);
    let mut first = value(&doc).unwrap();
    assert!(first["content"].as_str().unwrap().ends_with(body));
    assert_eq!(first["kind"], "released");
    assert_eq!(first["title"], "vpt released copy");
    assert_eq!(first["provenance"]["open_flags"], 2);
    assert_eq!(first["provenance"]["reviewed"], false);
    doc.generated_at.secs += 1;
    let second = value(&doc).unwrap();
    assert_ne!(first["generated_at"], second["generated_at"]);
    first["generated_at"] = second["generated_at"].clone();
    assert_eq!(first, second);
}
#[test]
fn handoff_rejects_inconsistent_review_metadata_and_identity_kind() {
    let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let mut doc = document(&id, "body");
    doc.kind = Kind::Brief;
    assert_eq!(value(&doc), Err(Error::Invalid));
    doc.kind = Kind::Released;
    doc.provenance.reviewed = true;
    assert_eq!(value(&doc), Err(Error::Invalid));
}
#[test]
fn handoff_bounds_raw_body_and_json_escape_expansion_before_output() {
    let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    assert_eq!(
        value(&document(&id, &"x".repeat(16 * 1024 * 1024 + 1))),
        Err(Error::CapacityExceeded)
    );
    assert_eq!(
        value(&document(&id, &"\0".repeat(6 * 1024 * 1024))),
        Err(Error::CapacityExceeded)
    );
}
