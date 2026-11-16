use crate::fix::fix;
use crate::fixture::{configured, detail, result, seed_seen, snapshot};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::SCHEMA_VERSION;

fn leaves_alone(
    name: &str,
    plant: impl FnOnce(&crate::support::Sandbox),
    code: i32,
) -> serde_json::Value {
    let sandbox = configured(name);
    fix(&sandbox, &[], 0);
    plant(&sandbox);
    let before = snapshot(&sandbox);
    let document = fix(&sandbox, &[], code);
    assert_eq!(document["fixed"], json!([]));
    assert_eq!(snapshot(&sandbox), before);
    document
}

#[test]
fn fix_leaves_note_versions_vpt_set_aside() {
    let document = leaves_alone(
        "doctor-fix-kept",
        |sandbox| {
            std::fs::write(
                sandbox
                    .path()
                    .join("home/.vpt/transcripts/.note.md.vpt-kept-41-2"),
                b"kept",
            )
            .unwrap();
        },
        1,
    );
    assert_eq!(result(&document, "leftovers", "kept_edits"), "issue");
}

#[test]
fn fix_leaves_the_mode_of_an_existing_state_folder() {
    let document = leaves_alone(
        "doctor-fix-mode",
        |sandbox| {
            let state = sandbox.path().join("state/vpt");
            std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755)).unwrap();
        },
        1,
    );
    assert_eq!(result(&document, "stores", "state_dir"), "issue");
    assert!(detail(&document, "stores", "state_dir").contains("run: chmod 700"));
}

#[test]
fn fix_leaves_a_home_link_that_points_elsewhere() {
    let document = leaves_alone(
        "doctor-fix-elsewhere",
        |sandbox| {
            let home = sandbox.path().join("home/.vpt");
            let moved = sandbox.path().join("data/moved");
            std::fs::rename(&home, &moved).unwrap();
            std::os::unix::fs::symlink(&moved, &home).unwrap();
            crate::observations_fixture::write_config(
                sandbox,
                &format!(
                    "symlink_target = {:?}\n[source]\nrecordings_dir = {:?}\n",
                    sandbox.path().join("data/wanted"),
                    sandbox.path().join("voice-memos/Recordings")
                ),
            );
        },
        1,
    );
    assert_eq!(result(&document, "stores", "home_link"), "issue");
}

#[test]
fn fix_leaves_an_older_ledger_schema_for_the_next_writing_command() {
    let document = leaves_alone(
        "doctor-fix-schema",
        |sandbox| {
            seed_seen(sandbox, false);
            rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db"))
                .unwrap()
                .pragma_update(None, "user_version", SCHEMA_VERSION - 1)
                .unwrap();
        },
        0,
    );
    assert_eq!(result(&document, "ledger", "schema"), "warning");
}
