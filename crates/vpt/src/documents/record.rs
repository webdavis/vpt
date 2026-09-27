use serde_json::{Value, json};
use vpt_application::ports::RecordingRecord;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

#[allow(
    clippy::result_large_err,
    reason = "CLI outcomes own the error document directly"
)]
pub(crate) fn record_json(record: &RecordingRecord) -> Result<Value, ErrorDocument> {
    let source_path = record
        .source_path
        .as_deref()
        .map(checked_path)
        .transpose()?;
    let audio_path = checked_path(record.audio_path.as_path())?;
    Ok(json!({
        "id": record.id.as_str(),
        "source_path": source_path,
        "digest": record.digest.hex(),
        "captured_at": record.captured_at.rfc3339_with(record.captured_offset),
        "captured_at_utc": record.captured_at.rfc3339(),
        "duration_secs": record.duration_secs,
        "title": record.title,
        "title_source": record.title_source.as_str(),
        "ingested_at": record.ingested_at.rfc3339(),
        "audio_path": audio_path,
        "stages": {
            "transcribe": record.stages.transcribe.as_str(),
            "note": record.stages.note.as_str(),
            "synthesis": record.stages.synthesis.as_str(),
        },
        "audio_trashed_at": record.audio_trashed_at.map(|at| at.rfc3339()),
    }))
}

#[allow(
    clippy::result_large_err,
    reason = "CLI outcomes own the error document directly"
)]
fn checked_path(path: &std::path::Path) -> Result<&str, ErrorDocument> {
    path.to_str()
        .ok_or_else(|| ErrorDocument::new(ErrorKind::Ledger, "record path is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use vpt_application::ports::{StageState, StageStates, TitleOrigin};
    use vpt_domain::digest::Sha256Digest;
    use vpt_domain::identity::RecordingId;
    use vpt_domain::time::{UtcInstant, UtcOffset};

    fn sample() -> RecordingRecord {
        let digest = Sha256Digest([0x4f; 32]);
        let captured = UtcInstant {
            secs: 1_787_604_456,
        };
        let offset = UtcOffset { secs: -21_600 };
        RecordingRecord {
            id: RecordingId::derive(captured, offset, &digest).unwrap(),
            source_path: Some("/owned/source.m4a".into()),
            digest,
            captured_at: captured,
            captured_offset: offset,
            duration_secs: 17,
            title: Some("Operator title".into()),
            title_source: TitleOrigin::VoiceMemos,
            ingested_at: UtcInstant {
                secs: 1_787_700_000,
            },
            audio_path: "/owned/audio.m4a".into(),
            stages: StageStates {
                transcribe: StageState::Succeeded,
                note: StageState::Failed,
                synthesis: StageState::Disabled,
            },
            audio_trashed_at: Some(UtcInstant {
                secs: 1_787_800_000,
            }),
        }
    }

    #[test]
    fn record_document_preserves_identity_times_title_paths_and_stages() {
        let record = sample();
        let value = record_json(&record).unwrap();
        assert_eq!(value["id"], record.id.as_str());
        assert_eq!(value["source_path"], "/owned/source.m4a");
        assert_eq!(value["digest"], record.digest.hex());
        assert_eq!(value["captured_at"], "2026-08-24T14:47:36-06:00");
        assert_eq!(value["captured_at_utc"], "2026-08-24T20:47:36Z");
        assert_eq!(value["duration_secs"], 17);
        assert_eq!(value["title"], "Operator title");
        assert_eq!(value["title_source"], "voice_memos");
        assert_eq!(value["ingested_at"], record.ingested_at.rfc3339());
        assert_eq!(value["audio_path"], "/owned/audio.m4a");
        assert_eq!(value["stages"]["transcribe"], "succeeded");
        assert_eq!(value["stages"]["note"], "failed");
        assert_eq!(value["stages"]["synthesis"], "disabled");
        assert_eq!(
            value["audio_trashed_at"],
            UtcInstant {
                secs: 1_787_800_000
            }
            .rfc3339()
        );
    }

    #[test]
    fn record_document_refuses_non_utf8_audio_path_without_leaking_bytes() {
        let mut invalid = sample();
        invalid.audio_path = OsString::from_vec(b"/owned/private-\xff".to_vec()).into();
        let error = record_json(&invalid).unwrap_err();
        assert_eq!(error.kind, vpt_protocol::error::ErrorKind::Ledger);
        assert!(!error.message.contains("private"));
    }

    #[test]
    fn record_document_refuses_non_utf8_source_path_without_leaking_bytes() {
        let mut invalid = sample();
        invalid.source_path = Some(OsString::from_vec(b"/owned/private-\xff".to_vec()).into());
        let error = record_json(&invalid).unwrap_err();
        assert_eq!(error.kind, vpt_protocol::error::ErrorKind::Ledger);
        assert!(!error.message.contains("private"));
    }
}
