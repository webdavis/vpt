#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "transcribe/lexical.rs"]
mod lexical;
#[path = "transcribe/output.rs"]
mod output;
#[path = "transcribe/pair_fixture.rs"]
mod pair_fixture;
#[path = "transcribe/pairs.rs"]
mod pairs;
#[path = "transcribe/protocol.rs"]
mod protocol;
#[path = "transcribe/recovery.rs"]
mod recovery;
mod support;
#[path = "transcribe/cases.rs"]
mod transcribe_cases;
#[path = "transcribe/uncertainty.rs"]
mod uncertainty;
#[path = "transcribe/whisply.rs"]
mod whisply;

use std::os::unix::fs::PermissionsExt;
use support::{Sandbox, stderr, stdout};
use vpt_adapters::{SqliteLedger, digest_open};
use vpt_application::ports::{
    PublicationJournal, RecordingLedger, StageState, StageStates, TranscriptLedger,
};

fn configured() -> (Sandbox, vpt_application::ports::RecordingRecord) {
    let sandbox = Sandbox::new("transcribe");
    let script = sandbox.path().join("bin/engine");
    std::fs::write(&script, "#!/bin/sh\nprintf '%s\\n' \"$1\" \"$2\" \"$3\" >> \"$4\"\n/bin/rm -- \"$1\"\n/bin/cat -- \"$5\"\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let document = sandbox.path().join("engine.json");
    std::fs::write(&document, serde_json::to_vec_pretty(&serde_json::json!({
        "schema":"vpt.engine/1", "engine":{"name":"custom", "tool":"fixture", "version":"1", "model":"test", "family":"independent"},
        "language":"fr-CA", "local":true, "segments":[{"start":0.0,"end":1.5,"text":"private transcript canary", "words":[{"text":"private transcript canary","start":0.0,"end":1.5}]}]
    })).unwrap()).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[recording]\nlanguage = 'fr-CA'\n[engines]\nmain = 'custom'\n[engines.custom]\nkind = 'command'\nfamily = 'independent'\nlocal = true\ncommand = [{:?}, '{{audio}}', '{{language}}', '{{out_dir}}', {:?}, {:?}]\n",
            script,
            sandbox.path().join("calls"),
            document
        ),
    );
    let mut row = fixture::record(&sandbox, 7, StageStates::fresh());
    let bytes = vpt_domain::fixtures::m4a(row.captured_at.secs, 12, b"archived private audio");
    std::fs::create_dir_all(row.audio_path.parent().unwrap()).unwrap();
    std::fs::write(&row.audio_path, &bytes).unwrap();
    row.digest = digest_open(&mut std::fs::File::open(&row.audio_path).unwrap()).unwrap();
    let old = row.audio_path.clone();
    row.id = vpt_domain::identity::RecordingId::derive(
        row.captured_at,
        row.captured_offset,
        &row.digest,
    )
    .unwrap();
    row.audio_path = old.parent().unwrap().join(format!("{}.m4a", row.id));
    std::fs::rename(old, &row.audio_path).unwrap();
    row.duration_secs = 12;
    fixture::seed(&sandbox, vec![row.clone()]);
    (sandbox, row)
}

fn run(sandbox: &Sandbox, id: &str, extra: &[&str]) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["transcribe", id, "--json"]).args(extra),
        std::time::Duration::from_secs(5),
    )
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).unwrap()
}

fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

#[path = "transcribe/notes.rs"]
mod notes;

#[path = "transcribe/alerts.rs"]
mod alerts;
