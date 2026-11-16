use crate::fix::fix;
use crate::fixture::{advice, configured, detail, result, run_json};
use crate::support::Sandbox;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;
use vpt_adapters::{RootDir, WriteLock};

fn staged(sandbox: &Sandbox) -> PathBuf {
    fix(sandbox, &[], 0);
    let path = sandbox.path().join("home/.vpt/audio/.vpt-staging-owned");
    std::fs::write(&path, b"a").unwrap();
    path
}

#[test]
fn fix_moves_staged_audio_to_the_trash_and_names_each_file() {
    let sandbox = configured("doctor-fix-staged-trash");
    let path = staged(&sandbox);
    let document = fix(&sandbox, &[], 0);
    assert_eq!(
        document["fixed"],
        json!([{"check": "leftovers", "name": "staged_audio",
            "detail": format!("moved {} to the Trash", path.display())}])
    );
    assert!(!path.exists());
    assert!(sandbox.path().join("trash/.vpt-staging-owned").exists());
    assert_eq!(detail(&document, "leftovers", "staged_audio"), "none");
    let again = fix(&sandbox, &["leftovers"], 0);
    assert_eq!(again["fixed"], json!([]));
    assert_eq!(detail(&again, "leftovers", "staged_audio"), "none");
}

#[test]
fn without_fix_staged_audio_stays_and_the_advice_names_the_command() {
    let sandbox = configured("doctor-staged-advice");
    let path = staged(&sandbox);
    let document = run_json(&sandbox, 1);
    assert!(path.exists());
    assert!(advice(&document, "leftovers", "staged_audio").contains("vpt doctor leftovers --fix"));
}

#[test]
fn fix_moves_nothing_while_another_writer_holds_the_lock() {
    let sandbox = configured("doctor-fix-staged-locked");
    let path = staged(&sandbox);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let document = fix(&sandbox, &["leftovers"], 1);
    assert_eq!(document["fixed"], json!([]));
    assert!(path.exists());
    assert_eq!(result(&document, "leftovers", "staged_audio"), "issue");
    assert!(
        detail(&document, "leftovers", "staged_audio").contains("another VPT writer is running")
    );
}
