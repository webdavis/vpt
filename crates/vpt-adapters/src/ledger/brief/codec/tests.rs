use super::*;
use vpt_domain::{
    brief::*,
    digest::Sha256Digest,
    occasion::{OccasionDetails, OccasionId, OccasionSource, Participant, time::OccasionTime},
    time::UtcInstant,
};

pub(crate) fn snapshot() -> Snapshot {
    let occasion = OccasionDetails {
        at: OccasionTime::parse("2026-10-01T09:00:00.123-06:00").unwrap(),
        title: "Private occasion".into(),
        duration_secs: Some(900),
        participants: vec![Participant {
            name: "Private person".into(),
            email: Some("private@example.test".into()),
        }],
        tags: vec!["planning".into()],
    };
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let record = crate::ledger::contract::record(1);
    let source = SourceDigest {
        recording: record.id.clone(),
        note: "/vault/transcripts/Private note.md".into(),
        digest: Sha256Digest([8; 32]),
    };
    Snapshot {
        id,
        source: OccasionSource::Dam,
        occasion,
        generated_at: UtcInstant { secs: 1000 },
        context: ContextSummary {
            calendar: ContextStatus::Failed {
                detail: "collector deadline".into(),
            },
            tasks: ContextStatus::Present,
        },
        tasks: vec![ContextTask {
            source: OccasionSource::Dam,
            project: "Private project".into(),
            id: "Private id".into(),
            content: "Private task".into(),
            due: Some("2026-10-02".into()),
            completed: false,
            priority: Some(4),
        }],
        pack: EvidencePack {
            selection: Selection {
                participants: vec![ParticipantMatch {
                    participant: 0,
                    term: TermState::Unconfirmed,
                    matched_note: None,
                    confirmation: Some("vpt confirm --term 'Private person'".into()),
                }],
                selected: vec![3],
                decisions: vec![Decision {
                    recording: record.id.clone(),
                    selected: true,
                    matched: Some(Match {
                        selector: Selector::Tag,
                        values: vec!["planning".into()],
                        anchors: vec![],
                    }),
                    rejected: None,
                }],
                omitted_notes: 0,
                rejected_notes: 0,
            },
            items: vec![Quote {
                text: "Private exact quote".into(),
                certainty: Certainty::Unverified,
                selector: Selector::Tag,
                source: QuoteSource {
                    recording: record.id,
                    at_secs: 0.25,
                    note: source.note.clone(),
                    digest: source.digest,
                },
            }],
            sources: vec![source],
            omitted_spans: 2,
            unresolved: Unresolved {
                flags: 1,
                notes_with_flags: 1,
                reviewed: false,
            },
        },
    }
}

#[test]
fn saved_brief_round_trips_the_complete_evidence_pack_without_private_debug_text() {
    let snapshot = snapshot();
    snapshot.document().validate().unwrap();
    let text = encode(&snapshot).unwrap();
    let decoded = decode(&text).unwrap();
    assert_eq!(decoded, snapshot);
    assert_eq!(
        vpt_protocol::brief::encode(&decoded.document()).unwrap(),
        vpt_protocol::brief::encode(&snapshot.document()).unwrap()
    );
    assert!(!format!("{decoded:?}").contains("Private"));
}

#[test]
fn malformed_and_inconsistent_saved_brief_data_fails_privately() {
    let text = encode(&snapshot()).unwrap();
    for text in [
        "Private invalid JSON".into(),
        text.replace("vpt.saved-brief/1", "vpt.saved-brief/9"),
        text.replace("unverified", "invented-certainty"),
    ] {
        let error = decode(&text).unwrap_err();
        assert!(!format!("{error:?}").contains("Private"));
    }
    let mut snapshot = snapshot();
    snapshot.pack.items[0].source.digest = Sha256Digest([99; 32]);
    assert!(encode(&snapshot).is_err());
}
