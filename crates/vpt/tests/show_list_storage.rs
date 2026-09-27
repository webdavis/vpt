#[path = "show_list_storage/active_writer.rs"]
mod active_writer;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
mod support;

use fixture::{configured, dirty, record, seed, snapshot, write_config};
use std::time::Duration;
use support::{run, stderr, stdout};
use vpt_adapters::{RootDir, WriteLock};
use vpt_application::ports::{StageState, StageStates};

fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).expect("one result document")
}

fn error(output: &std::process::Output, code: i32, kind: &str) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    assert!(output.stdout.is_empty());
    let document: serde_json::Value =
        serde_json::from_str(&stderr(output)).expect("one error document");
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], kind);
    document
}

#[test]
fn show_routes_to_the_recorded_row_and_preserves_its_fields() {
    let sandbox = configured("show-known");
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    let result = json(&run(sandbox.vpt().args([
        "show",
        row.id.as_str(),
        "--json",
    ])));
    assert_eq!(result["schema"], "vpt.result/1");
    assert_eq!(result["command"], "show");
    assert_eq!(result["id"], row.id.as_str());
    assert_eq!(result["title"], "title-1");
    assert_eq!(result["stages"]["transcribe"], "pending");
    assert_eq!(result["audio_path"], row.audio_path.to_str().unwrap());
}

#[test]
fn show_unknown_id_is_usage_with_the_requested_identity() {
    let sandbox = configured("show-unknown");
    let id = "2026-08-24T144736-4f3ab19c02de";
    let output = run(sandbox.vpt().args(["show", id, "--json"]));
    let failure = error(&output, 2, "usage");
    assert_eq!(failure["error"]["ids"], serde_json::json!([id]));
}

#[test]
fn show_malformed_id_is_usage() {
    let sandbox = configured("show-malformed");
    let output = run(sandbox.vpt().args(["show", "nonsense", "--json"]));
    error(&output, 2, "usage");
}

#[test]
fn list_routes_to_every_record_in_capture_order() {
    let sandbox = configured("list-all");
    let second = record(&sandbox, 2, StageStates::fresh());
    let first = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![second.clone(), first.clone()]);
    let output = run(sandbox.vpt().args(["list", "--json"]));
    let result = json(&output);
    assert_eq!(result["command"], "list");
    assert_eq!(result["recordings"][0]["id"], first.id.as_str());
    assert_eq!(result["recordings"][1]["id"], second.id.as_str());
}

fn stage_case(stage: &str) -> (String, serde_json::Value) {
    let sandbox = configured(stage);
    let first = record(
        &sandbox,
        1,
        StageStates {
            transcribe: StageState::Succeeded,
            note: StageState::Pending,
            synthesis: StageState::Disabled,
        },
    );
    let second = record(
        &sandbox,
        2,
        StageStates {
            transcribe: StageState::Failed,
            note: StageState::Succeeded,
            synthesis: StageState::Pending,
        },
    );
    let done = record(
        &sandbox,
        3,
        StageStates {
            transcribe: StageState::Succeeded,
            note: StageState::Succeeded,
            synthesis: StageState::Succeeded,
        },
    );
    seed(&sandbox, vec![done, first, second]);
    let result = json(&run(sandbox
        .vpt()
        .args(["list", "--stage", stage, "--json"])));
    (result["command"].as_str().unwrap().to_owned(), result)
}

#[test]
fn list_transcribe_includes_failed_but_not_succeeded() {
    let (command, result) = stage_case("transcribe");
    assert_eq!(command, "list");
    assert_eq!(result["recordings"].as_array().unwrap().len(), 1);
    assert_eq!(result["recordings"][0]["title"], "title-2");
}

#[test]
fn list_note_includes_pending_but_not_succeeded() {
    let (_, result) = stage_case("note");
    assert_eq!(result["recordings"].as_array().unwrap().len(), 1);
    assert_eq!(result["recordings"][0]["title"], "title-1");
}

#[test]
fn list_synthesis_includes_disabled_and_pending() {
    let (_, result) = stage_case("synthesis");
    assert_eq!(result["recordings"].as_array().unwrap().len(), 2);
    assert_eq!(result["recordings"][0]["title"], "title-1");
    assert_eq!(result["recordings"][1]["title"], "title-2");
}

#[test]
fn list_rejects_unrecognized_stage_words() {
    let sandbox = configured("list-invalid");
    let output = run(sandbox.vpt().args(["list", "--stage", "polish", "--json"]));
    error(&output, 2, "usage");
}

#[test]
fn storage_reports_seven_roots_and_real_file_metadata() {
    let sandbox = configured("storage-seven");
    let audio = sandbox.path().join("home/.vpt/audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join("synthetic.m4a"), b"abc").unwrap();
    let result = json(&run(sandbox.vpt().args(["storage", "--json"])));
    assert_eq!(result["command"], "storage");
    assert_eq!(result["stores"].as_object().unwrap().len(), 7);
    assert_eq!(result["stores"]["audio"]["files"], 1);
    assert_eq!(result["stores"]["audio"]["bytes"], 3);
    assert!(result["stores"]["audio"]["oldest"].is_string());
    assert_eq!(result["stores"]["released"]["files"], 0);
    assert!(result["stores"]["released"]["oldest"].is_null());
}

#[test]
fn fresh_list_does_not_create_state_or_stores() {
    let sandbox = configured("fresh-list");
    let before = snapshot(&sandbox);
    let result = json(&run(sandbox.vpt().args(["list", "--json"])));
    assert_eq!(result["recordings"].as_array().unwrap().len(), 0);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn fresh_storage_does_not_create_state_or_stores() {
    let sandbox = configured("fresh-storage");
    let before = snapshot(&sandbox);
    let result = json(&run(sandbox.vpt().args(["storage", "--json"])));
    assert_eq!(result["stores"].as_object().unwrap().len(), 7);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn fresh_show_reports_unknown_without_creating_state() {
    let sandbox = configured("fresh-show");
    let before = snapshot(&sandbox);
    let output = run(sandbox
        .vpt()
        .args(["show", "2026-08-24T144736-4f3ab19c02de", "--json"]));
    error(&output, 2, "usage");
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn show_preserves_dirty_ledger_and_store_bytes_modes_and_mtimes() {
    let sandbox = configured("dirty-show");
    let row = record(&sandbox, 1, StageStates::fresh());
    dirty(&sandbox, row.clone());
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args([
        "show",
        row.id.as_str(),
        "--json",
    ])));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn list_preserves_dirty_ledger_and_store_bytes_modes_and_mtimes() {
    let sandbox = configured("dirty-list");
    let row = record(&sandbox, 1, StageStates::fresh());
    dirty(&sandbox, row);
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args(["list", "--json"])));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn list_after_the_fixture_writer_exits_changes_no_state() {
    let sandbox = configured("list-after-writer-exit");
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    let before = snapshot(&sandbox);
    let result = json(&run(sandbox.vpt().args(["list", "--json"])));
    assert_eq!(result["command"], "list");
    assert_eq!(result["recordings"][0]["id"], row.id.as_str());
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn storage_preserves_dirty_ledger_and_store_bytes_modes_and_mtimes() {
    let sandbox = configured("dirty-storage");
    let row = record(&sandbox, 1, StageStates::fresh());
    dirty(&sandbox, row);
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args(["storage", "--json"])));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn show_needs_neither_recorder_source_nor_valid_store_roots() {
    let sandbox = configured("show-ledger-only");
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    std::fs::rename(
        sandbox.path().join("voice-memos"),
        sandbox.path().join("held-source"),
    )
    .unwrap();
    let regular = sandbox.path().join("regular-audio");
    std::fs::write(&regular, b"not a store directory").unwrap();
    write_config(&sandbox, &format!("[stores]\naudio = {:?}\n", regular));
    error(&run(sandbox.vpt().args(["storage", "--json"])), 2, "config");
    let result = json(&run(sandbox.vpt().args([
        "show",
        row.id.as_str(),
        "--json",
    ])));
    assert_eq!(result["id"], row.id.as_str());
}

#[test]
fn list_needs_neither_recorder_source_nor_valid_store_roots() {
    let sandbox = configured("list-ledger-only");
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    std::fs::rename(
        sandbox.path().join("voice-memos"),
        sandbox.path().join("held-source"),
    )
    .unwrap();
    let regular = sandbox.path().join("regular-audio");
    std::fs::write(&regular, b"not a store directory").unwrap();
    write_config(&sandbox, &format!("[stores]\naudio = {:?}\n", regular));
    error(&run(sandbox.vpt().args(["storage", "--json"])), 2, "config");
    let result = json(&run(sandbox.vpt().args(["list", "--json"])));
    assert_eq!(result["recordings"][0]["id"], row.id.as_str());
}

#[test]
fn storage_needs_neither_recorder_source_nor_readable_ledger() {
    let sandbox = configured("storage-only");
    let row = record(&sandbox, 1, StageStates::fresh());
    std::fs::create_dir_all(sandbox.path().join("state/vpt")).unwrap();
    let audio = sandbox.path().join("home/.vpt/audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join("synthetic.m4a"), b"abc").unwrap();
    std::fs::rename(
        sandbox.path().join("voice-memos"),
        sandbox.path().join("held-source"),
    )
    .unwrap();
    let state = sandbox.path().join("state/vpt");
    std::fs::write(state.join("vpt.db"), b"corrupt ledger").unwrap();
    assert!(!state.join("vpt.db-wal").exists());
    assert!(!state.join("vpt.db-shm").exists());
    let before = snapshot(&sandbox);
    let probe = run(sandbox.vpt().args(["show", row.id.as_str(), "--json"]));
    let refusal = error(&probe, 1, "ledger");
    assert_eq!(refusal["error"]["message"], "ledger is corrupt");
    assert_eq!(snapshot(&sandbox), before);
    let result = json(&run(sandbox.vpt().args(["storage", "--json"])));
    assert_eq!(result["stores"]["audio"]["bytes"], 3);
    assert_eq!(snapshot(&sandbox), before);
}

fn missing_companion(command: &str, companion: &str) {
    let sandbox = configured(&format!("{command}-missing-{companion}"));
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    let companion_path = sandbox.path().join("state/vpt").join(companion);
    let held = sandbox.path().join(format!("held-{companion}"));
    std::fs::rename(&companion_path, &held).unwrap();
    let held_bytes = std::fs::read(&held).unwrap();
    let before = snapshot(&sandbox);
    let mut args = vec![command, "--json"];
    if command == "show" {
        args.insert(1, row.id.as_str());
    }
    let output = run(sandbox.vpt().args(args));
    let failure = error(&output, 1, "ledger");
    assert_eq!(failure["schema"], "vpt.error/1");
    assert!(!stderr(&output).contains("title-1"));
    assert!(!stderr(&output).contains(row.id.as_str()));
    assert_eq!(snapshot(&sandbox), before);
    assert_eq!(std::fs::read(&held).unwrap(), held_bytes);
}

#[test]
fn show_with_populated_ledger_missing_wal_reports_error_without_repair() {
    missing_companion("show", "vpt.db-wal");
}

#[test]
fn list_with_populated_ledger_missing_wal_reports_error_without_repair() {
    missing_companion("list", "vpt.db-wal");
}

#[test]
fn show_with_populated_ledger_missing_shm_reports_error_without_repair() {
    missing_companion("show", "vpt.db-shm");
}

#[test]
fn list_with_populated_ledger_missing_shm_reports_error_without_repair() {
    missing_companion("list", "vpt.db-shm");
}

#[test]
fn observations_complete_while_application_write_lock_is_held() {
    let sandbox = configured("observations-held-write-lock");
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    let state = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _held = WriteLock::acquire(&state, Duration::ZERO).unwrap();
    let before = snapshot(&sandbox);
    for args in [
        vec!["show", row.id.as_str(), "--json"],
        vec!["list", "--json"],
        vec!["storage", "--json"],
    ] {
        json(&run(sandbox.vpt().args(args)));
        assert_eq!(snapshot(&sandbox), before);
    }
}

#[test]
fn show_refuses_linked_ledger_without_following_it() {
    linked_ledger_leaf("show", "vpt.db", "path escapes a checked root");
}

#[test]
fn list_refuses_linked_ledger_without_following_it() {
    linked_ledger_leaf("list", "vpt.db", "path escapes a checked root");
}

#[test]
fn show_refuses_linked_wal_without_following_it() {
    linked_ledger_leaf(
        "show",
        "vpt.db-wal",
        "ledger path escapes its retained state root",
    );
}

#[test]
fn list_refuses_linked_shm_without_following_it() {
    linked_ledger_leaf(
        "list",
        "vpt.db-shm",
        "ledger path escapes its retained state root",
    );
}

fn linked_ledger_leaf(command: &str, leaf: &str, message: &str) {
    let sandbox = configured(&format!("{command}-linked-{leaf}"));
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    let checked = sandbox.path().join("state/vpt").join(leaf);
    let held = sandbox.path().join(format!("held-{leaf}"));
    std::fs::rename(&checked, &held).unwrap();
    let before = std::fs::read(&held).unwrap();
    std::os::unix::fs::symlink(&held, &checked).unwrap();
    let tree_before = snapshot(&sandbox);
    let mut args = vec![command, "--json"];
    if command == "show" {
        args.insert(1, row.id.as_str());
    }
    let output = run(sandbox.vpt().args(args));
    let failure = error(&output, 3, "refused");
    assert_eq!(
        failure,
        serde_json::json!({
            "schema": "vpt.error/1",
            "error": {
                "kind": "refused",
                "rule": "path_escape",
                "message": message,
                "ids": [],
                "completed": [],
            }
        })
    );
    assert_eq!(std::fs::read(&held).unwrap(), before);
    assert_eq!(snapshot(&sandbox), tree_before);
}
