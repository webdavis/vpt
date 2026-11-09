use super::*;
use std::fs;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::RecordingLedger;

#[test]
fn empty_previously_populated_source_aborts_once_per_run_and_preserves_archive() {
    let sandbox = configured();
    let source = add(&sandbox, "gone.m4a");
    let ingested = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let archive = std::path::Path::new(ingested["ingested"][0]["audio_path"].as_str().unwrap());
    let bytes = fs::read(archive).unwrap();
    fs::rename(&source, sandbox.path().join("held-source.m4a")).unwrap();
    for json_output in [true, false] {
        let mut command = sandbox.vpt();
        command.arg("ingest");
        if json_output {
            command.arg("--json");
        }
        let output = run(&mut command);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let text = stderr(&output);
        assert!(text.contains("previously populated Voice Memos source is empty"));
        if json_output {
            let error: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(error["schema"], "vpt.error/1");
            assert_eq!(error["error"]["kind"], "store");
            assert_eq!(error["error"]["completed"], serde_json::json!([]));
        } else {
            assert!(text.contains("Ingestion stopped; 0 recording(s) committed"));
        }
        assert_eq!(fs::read(archive).unwrap(), bytes);
        let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap();
        assert_eq!(ledger.recordings().unwrap().len(), 1);
        let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
        assert_eq!(
            events
                .lines()
                .filter(|line| line.contains("\"event\":\"ingest_failed\""))
                .count(),
            if json_output { 1 } else { 2 }
        );
        assert!(!events.contains("private audio"));
    }
}

#[test]
fn unreadable_title_database_continues_untitled_without_changing_voice_memos() {
    let sandbox = configured();
    let source = add(&sandbox, "untitled.m4a");
    let voice_memos = sandbox.path().join("voice-memos");
    fs::write(
        voice_memos.join("CloudRecordings.db"),
        b"private invalid title database",
    )
    .unwrap();
    let before = snapshot(&sandbox);
    let result = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(result["ingested"].as_array().unwrap().len(), 1);
    assert!(result["ingested"][0]["title"].is_null());
    assert_eq!(result["ingested"][0]["title_source"], "unavailable");
    assert_eq!(result["deferred"], serde_json::json!([]));
    assert!(
        !result
            .to_string()
            .contains("private invalid title database")
    );
    let path = result["ingested"][0]["audio_path"].as_str().unwrap();
    assert_eq!(fs::read(path).unwrap(), fs::read(source).unwrap());
    assert_eq!(
        before
            .into_iter()
            .filter(|entry| entry.is_within(&voice_memos))
            .collect::<Vec<_>>(),
        snapshot(&sandbox)
            .into_iter()
            .filter(|entry| entry.is_within(&voice_memos))
            .collect::<Vec<_>>()
    );
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("ingest_failed"));
    assert!(!events.contains("private invalid title database"));
}
