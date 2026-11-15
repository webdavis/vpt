use crate::support::stderr;
use crate::{backup, deliver, ingested, machine, point_at, recordings, state};

const FIRST: i64 = 1_787_604_456;

#[test]
fn a_later_write_from_another_machine_is_accepted_silently() {
    let mine = machine("identity-later-mine");
    let theirs = machine("identity-later-theirs");
    ingested(&mine, "a.m4a", FIRST);
    deliver(&state(&mine).join("vpt.db"), &state(&theirs));
    ingested(&theirs, "b.m4a", FIRST + 60);
    deliver(&state(&theirs).join("vpt.db"), &state(&mine));
    let listed = crate::vpt(&mine, &["list"]);
    assert!(listed.stderr.is_empty(), "{}", stderr(&listed));
    ingested(&mine, "c.m4a", FIRST + 120);
    assert_eq!(recordings(&mine), 3);
    deliver(&state(&mine).join("vpt.db"), &state(&theirs));
    ingested(&theirs, "d.m4a", FIRST + 180);
    assert_eq!(recordings(&theirs), 4);
}

#[test]
fn a_new_machine_writes_to_a_ledger_it_has_never_seen() {
    let mine = machine("identity-new-mine");
    let theirs = machine("identity-new-theirs");
    ingested(&theirs, "a.m4a", FIRST);
    ingested(&theirs, "b.m4a", FIRST + 60);
    deliver(&state(&theirs).join("vpt.db"), &state(&mine));
    ingested(&mine, "c.m4a", FIRST + 120);
    assert_eq!(recordings(&mine), 3);
}

#[test]
fn a_machine_whose_marker_was_deleted_is_treated_as_new() {
    let sandbox = machine("identity-marker-deleted");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    std::fs::remove_dir_all(state(&sandbox).join("markers")).unwrap();
    ingested(&sandbox, "c.m4a", FIRST + 120);
}

#[test]
fn a_changed_state_folder_path_is_treated_as_a_new_machine() {
    let sandbox = machine("identity-path-changed");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    let moved = sandbox.path().join("elsewhere/vpt");
    deliver(&older, &moved);
    point_at(&sandbox, &moved);
    ingested(&sandbox, "c.m4a", FIRST + 120);
}
