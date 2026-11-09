use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::{SqliteLedger, digest_open};
use vpt_application::ports::RecordingLedger;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

#[test]
fn a_source_rename_and_timezone_change_keep_identity_then_an_edit_creates_a_new_record() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let source = add(&sandbox, "original.m4a");
    let first = json(&run(sandbox
        .vpt()
        .args(["ingest", "--json"])
        .env("TZ", "UTC")));
    let id = RecordingId::parse(first["ingested"][0]["id"].as_str().unwrap()).unwrap();
    let original = ledger(&sandbox).by_id(&id).unwrap().unwrap();
    assert_eq!(original.captured_offset.secs, 0);
    let archive = fs::read(&original.audio_path).unwrap();
    let renamed = source.with_file_name("renamed.m4a");
    fs::rename(&source, &renamed).unwrap();
    let second = json(&run(sandbox
        .vpt()
        .args(["ingest", "--json"])
        .env("TZ", "Pacific/Kiritimati")));
    assert_eq!(second["ingested"], serde_json::json!([]));
    assert_eq!(second["already_ingested"], serde_json::json!([id.as_str()]));
    let rows = ledger(&sandbox).recordings().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, id);
    assert_eq!(rows[0].digest, original.digest);
    assert_eq!(rows[0].audio_path, original.audio_path);
    assert_eq!(rows[0].captured_offset, original.captured_offset);
    assert_eq!(rows[0].source_path.as_ref(), Some(&renamed));
    assert!(
        ledger(&sandbox)
            .seen(&source)
            .unwrap()
            .unwrap()
            .source_gone_at
            .is_some()
    );
    assert_eq!(
        ledger(&sandbox).seen(&renamed).unwrap().unwrap().recording,
        Some(id.clone())
    );
    assert_eq!(
        fs::read_dir(sandbox.path().join("trash")).unwrap().count(),
        1
    );
    assert_eq!(
        fs::read_dir(original.audio_path.parent().unwrap())
            .unwrap()
            .count(),
        1
    );

    let changed = m4a(1_787_604_456, 7, b"edited audio with different bytes");
    fs::write(&renamed, &changed).unwrap();
    fs::File::open(&renamed)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    let third = json(&run(sandbox
        .vpt()
        .args(["ingest", "--json"])
        .env("TZ", "Pacific/Kiritimati")));
    let new_id = RecordingId::parse(third["ingested"][0]["id"].as_str().unwrap()).unwrap();
    assert_ne!(new_id, id);
    let edited = ledger(&sandbox).by_id(&new_id).unwrap().unwrap();
    assert_eq!(edited.captured_offset.secs, 50_400);
    assert_eq!(edited.duration_secs, 7);
    assert_eq!(fs::read(&edited.audio_path).unwrap(), changed);
    assert_eq!(fs::read(&original.audio_path).unwrap(), archive);
    assert_eq!(ledger(&sandbox).recordings().unwrap().len(), 2);
    assert_eq!(
        ledger(&sandbox).seen(&renamed).unwrap().unwrap().recording,
        Some(new_id)
    );
}

#[test]
fn orphan_recovery_preserves_archived_offset_with_or_without_a_surviving_source() {
    for survives in [false, true] {
        let sandbox = configured();
        sandbox.install_fake_helper();
        let source = add(&sandbox, "original.m4a");
        let bytes = fs::read(&source).unwrap();
        let digest = digest_open(&mut fs::File::open(&source).unwrap()).unwrap();
        let at = UtcInstant {
            secs: 1_787_604_456,
        };
        let offset = UtcOffset { secs: -21_600 };
        let id = RecordingId::derive(at, offset, &digest).unwrap();
        let directory = sandbox.path().join("home/.vpt/audio");
        fs::create_dir_all(&directory).unwrap();
        let archive = directory.join(format!("{id}.m4a"));
        fs::write(&archive, &bytes).unwrap();
        fs::set_permissions(&archive, fs::Permissions::from_mode(0o600)).unwrap();
        if !survives {
            fs::remove_file(&source).unwrap();
        }
        let before = snapshot(&sandbox);
        let preview = json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
        assert_eq!(preview["recovered"], serde_json::json!([]));
        assert_eq!(snapshot(&sandbox), before);
        let result = json(&run(sandbox
            .vpt()
            .args(["ingest", "--json"])
            .env("TZ", "Pacific/Kiritimati")));
        assert_eq!(result["recovered"], serde_json::json!([id.as_str()]));
        assert_eq!(result["ingested"], serde_json::json!([]));
        let rows = ledger(&sandbox).recordings().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].captured_at, at);
        assert_eq!(rows[0].captured_offset, offset);
        assert_eq!(rows[0].digest, digest);
        assert_eq!(rows[0].duration_secs, 12);
        assert_eq!(rows[0].audio_path, archive);
        assert_eq!(rows[0].source_path, survives.then_some(source));
        assert_eq!(fs::read(&archive).unwrap(), bytes);
        assert_eq!(
            fs::metadata(&archive).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        assert_eq!(
            fs::read_dir(sandbox.path().join("trash")).unwrap().count(),
            usize::from(survives)
        );
        let repeated = json(&run(sandbox.vpt().args(["ingest", "--json"])));
        assert_eq!(repeated["recovered"], serde_json::json!([]));
        assert_eq!(ledger(&sandbox).recordings().unwrap().len(), 1);
    }
}

#[test]
fn archive_collision_refuses_without_replacing_bytes_or_inserting_rows() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let source = add(&sandbox, "a.m4a");
    let bytes = fs::read(&source).unwrap();
    let digest = digest_open(&mut fs::File::open(&source).unwrap()).unwrap();
    let id = RecordingId::derive(
        UtcInstant {
            secs: 1_787_604_456,
        },
        UtcOffset { secs: 0 },
        &digest,
    )
    .unwrap();
    let directory = sandbox.path().join("home/.vpt/audio");
    fs::create_dir_all(&directory).unwrap();
    let archive = directory.join(format!("{id}.m4a"));
    // A valid foreign container still cannot occupy this recording's archive path.
    let foreign = m4a(1_787_604_456, 12, b"foreign audio");
    fs::write(&archive, &foreign).unwrap();
    fs::set_permissions(&archive, fs::Permissions::from_mode(0o600)).unwrap();
    let output = run(sandbox.vpt().args(["ingest", "--json"]).env("TZ", "UTC"));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "archive_collision");
    assert_eq!(fs::read(&archive).unwrap(), foreign);
    assert_eq!(fs::read(&source).unwrap(), bytes);
    assert!(ledger(&sandbox).recordings().unwrap().is_empty());
    assert!(ledger(&sandbox).seen(&source).unwrap().is_none());
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert_eq!(events.matches("\"event\":\"ingest_failed\"").count(), 1);
    assert!(!events.contains("foreign audio"));
    assert!(!events.contains("private audio"));
}
