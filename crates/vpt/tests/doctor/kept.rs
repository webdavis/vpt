use crate::fixture::{check, configured, detail, run_json};
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::{RootDir, SqliteLedger};

const INTENDED: &str = "09e886debb5a0412d1899d253b2e4514d396b1e63ff219fda2dbc4a6cc35b386";
const OLD: &str = "cba06b5736faf67e54b07b561eae94395e774c517a7d910a54369e1263ccfbd4";

#[test]
fn vpts_own_leftovers_beside_a_pending_note_are_not_reported_as_edits() {
    let sandbox = configured("doctor-kept-leftovers");
    let transcripts = sandbox.path().join("home/.vpt/transcripts");
    std::fs::create_dir_all(&transcripts).unwrap();
    std::fs::write(transcripts.join(".note.md.vpt-kept-41-1"), b"intended").unwrap();
    std::fs::write(transcripts.join(".note.md.vpt-kept-41-2"), b"old").unwrap();
    std::fs::write(transcripts.join(".note.md.vpt-kept-41-3"), b"operator").unwrap();
    let state = sandbox.path().join("state/vpt");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    SqliteLedger::open(&RootDir::open(&state).unwrap()).unwrap();
    let connection = rusqlite::Connection::open(state.join("vpt.db")).unwrap();
    connection
        .execute(
            "INSERT INTO dirty_publications (target, expected_previous, intended, recorded_at) \
             VALUES (?1, ?2, ?3, 1)",
            [
                transcripts.join("note.md").to_string_lossy().as_ref(),
                OLD,
                INTENDED,
            ],
        )
        .unwrap();
    let checks = run_json(&sandbox, 3);
    drop(connection);
    assert_eq!(check(&checks, "kept_edits")["ok"], false);
    assert_eq!(
        detail(&checks, "kept_edits"),
        "1 version vpt kept beside notes: transcripts/.note.md.vpt-kept-41-3; keep what you \
         need in each note, then delete the kept file"
    );
}
