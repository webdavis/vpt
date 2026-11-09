use super::*;
mod tasks;
use vpt_domain::{
    brief::*, digest::Sha256Digest, identity::RecordingId, occasion::time::OccasionTime, tags,
    time::UtcOffset, transcript::*,
};

fn row() -> Candidate {
    Candidate {
        recording: RecordingId::derive(
            UtcInstant { secs: 100 },
            UtcOffset { secs: 0 },
            &Sha256Digest([3; 32]),
        )
        .unwrap(),
        captured_at: UtcInstant { secs: 100 },
        source: Some(TranscriptSource {
            note: "/notes/private.md".into(),
            digest: Sha256Digest([8; 32]),
            flags: vec![],
            reviewed: false,
            transcript: Transcript {
                engine: EngineMetadata {
                    name: "test".into(),
                    tool: "test".into(),
                    version: "1".into(),
                    model: "test".into(),
                    family: "test".into(),
                },
                language: "en".into(),
                language_source: LanguageSource::Reported,
                local: true,
                segments: vec![Segment {
                    start: 0.25,
                    end: 1.0,
                    text: "Private exact quote".into(),
                    words: vec![],
                }],
            },
        }),
        relations: vec![],
        tags: vec![tags::Tag {
            name: "planning".into(),
            state: tags::State::Confirmed,
            provenance: tags::Provenance::Manual,
            updated_at: None,
        }],
    }
}
fn occasion() -> OccasionDetails {
    OccasionDetails {
        at: OccasionTime::parse("2026-10-01T09:00:00.123-06:00").unwrap(),
        title: "Private occasion".into(),
        duration_secs: Some(900),
        participants: vec![],
        tags: vec!["planning".into()],
    }
}
#[test]
fn brief_json_has_exact_quotes_required_certainty_sources_and_independent_context_statuses() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let rows = [row()];
    let pack = assemble(&Request {
        occasion: &occasion,
        terms: &[],
        candidates: &rows,
        now: UtcInstant { secs: 1000 },
        options: Options::default(),
    })
    .unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Present,
        tasks: ContextStatus::Unsupported,
    };
    let text = encode(&Document {
        id: &id,
        source: OccasionSource::Manual,
        occasion: &occasion,
        pack: &pack,
        context: &context,
        tasks: &[],
        generated_at: UtcInstant { secs: 1000 },
    })
    .unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["schema"], SCHEMA);
    assert_eq!(json["occasion"]["at"], occasion.at.as_str());
    assert_eq!(json["items"][0]["text"], "Private exact quote");
    assert_eq!(json["items"][0]["certainty"], "unverified");
    assert_eq!(
        json["items"][0]["sources"][0]["recording"],
        rows[0].recording.as_str()
    );
    assert_eq!(json["items"][0]["sources"][0]["at_secs"], 0.25);
    assert_eq!(json["items"][0]["sources"][0]["note"], "/notes/private.md");
    assert_eq!(json["items"][0]["selector"], "tag");
    assert_eq!(json["unresolved"]["reviewed"], false);
    assert_eq!(
        json["context"]["calendar"],
        serde_json::json!({"status":"present","reason":null,"detail":null})
    );
    assert_eq!(
        json["context"]["tasks"],
        serde_json::json!({"status":"absent","reason":"unsupported","detail":null})
    );
    assert_eq!(json["generated_at"], "1970-01-01T00:16:40Z");
}
#[test]
fn an_item_missing_certainty_or_sources_fails_serialization() {
    let row = row();
    let sources = [Source {
        recording: &row.recording,
        at_secs: 0.25,
        note: std::path::Path::new("/notes/private.md"),
    }];
    let item = Item {
        text: "Private quote",
        certainty: None,
        sources: &sources,
        selector: Selector::Tag,
    };
    assert!(serde_json::to_string(&item).is_err());
    let item = Item {
        certainty: Some(Certainty::Confirmed),
        sources: &[],
        ..item
    };
    assert!(serde_json::to_string(&item).is_err());
    let item = Item {
        sources: &sources,
        ..item
    };
    assert_eq!(
        serde_json::to_value(&item).unwrap()["certainty"],
        "confirmed"
    );
}

#[test]
fn forged_sources_counts_and_invalid_timestamps_fail_without_private_diagnostics() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let rows = [row()];
    let pack = assemble(&Request {
        occasion: &occasion,
        terms: &[],
        candidates: &rows,
        now: UtcInstant { secs: 1000 },
        options: Options::default(),
    })
    .unwrap();
    let context = ContextSummary::default();
    for case in 0..6 {
        let mut changed = pack.clone();
        match case {
            0 => changed.items[0].source.digest = Sha256Digest([99; 32]),
            1 => changed.items[0].source.note = "/other/private.md".into(),
            2 => changed.items[0].source.at_secs = f64::NAN,
            3 => changed.selection.omitted_notes += 1,
            4 => changed.unresolved.notes_with_flags = 2,
            _ => changed
                .selection
                .selected
                .push(changed.selection.selected[0]),
        }
        let result = encode(&Document {
            id: &id,
            source: OccasionSource::Manual,
            occasion: &occasion,
            pack: &changed,
            context: &context,
            tasks: &[],
            generated_at: UtcInstant { secs: 1000 },
        });
        assert!(result.is_err(), "case {case}");
        assert!(!format!("{result:?}").contains("Private"));
    }
}

#[test]
fn unconfirmed_participants_and_omitted_notes_remain_visible_in_json() {
    let mut occasion = occasion();
    occasion
        .participants
        .push(vpt_domain::occasion::Participant {
            name: "Private person".into(),
            email: None,
        });
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let rows = [row()];
    let pack = assemble(&Request {
        occasion: &occasion,
        terms: &[],
        candidates: &rows,
        now: UtcInstant { secs: 1000 },
        options: Options {
            max_notes: 0,
            ..Default::default()
        },
    })
    .unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Failed {
            detail: "collector deadline".into(),
        },
        tasks: ContextStatus::Disabled,
    };
    let text = encode(&Document {
        id: &id,
        source: OccasionSource::Manual,
        occasion: &occasion,
        pack: &pack,
        context: &context,
        tasks: &[],
        generated_at: UtcInstant { secs: 1000 },
    })
    .unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["occasion"]["participants"][0]["term"], "unconfirmed");
    assert!(json["occasion"]["participants"][0]["matched_note"].is_null());
    assert_eq!(
        json["not_selected"][0]["command"],
        "vpt confirm --term 'Private person'"
    );
    assert_eq!(json["not_selected"][1]["reason"], "note-limit");
    assert_eq!(json["selection"]["omitted_notes"], 1);
    assert_eq!(json["selection"]["candidates"][0]["selector"], "tag");
    assert_eq!(json["context"]["calendar"]["reason"], "failed");
    assert_eq!(json["context"]["tasks"]["reason"], "disabled");
}
