use crate::fixture::snapshot;
use crate::support::{stderr, stdout};
use crate::{assert_refused, backup, deliver, ingest, ingested, machine, recordings, state, vpt};

const FIRST: i64 = 1_787_604_456;

#[test]
fn accept_preview_names_both_sides_and_changes_nothing() {
    let sandbox = machine("identity-preview");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    let before = snapshot(&sandbox);
    let preview = crate::json(&sandbox, &["ledger", "accept", "--preview", "-o", "json"]);
    assert_eq!(snapshot(&sandbox), before);
    assert_eq!(preview["command"], "ledger accept");
    assert_eq!(preview["preview"], true);
    assert_eq!(preview["finding"]["rule"], "ledger_behind");
    assert_eq!(preview["database"]["write_counter"], 1);
    assert_eq!(preview["database"]["recordings"], 1);
    assert_eq!(preview["this_machine"]["write_counter"], 2);
    assert_eq!(
        preview["database"]["id"],
        preview["this_machine"]["database_id"]
    );
    let human = stdout(&crate::succeeds(
        &sandbox,
        &["ledger", "accept", "--preview"],
    ));
    assert!(human.contains("write_counter: 1"), "{human}");
    assert!(human.contains("this_machine:"), "{human}");
    assert_eq!(snapshot(&sandbox), before);
    let refused = ingest(&sandbox, "c.m4a", FIRST + 120);
    assert_refused(&refused, "ledger_behind", &state(&sandbox));
}

#[test]
fn accept_with_nothing_to_accept_exits_two_and_changes_nothing() {
    let sandbox = machine("identity-nothing");
    for args in [
        ["ledger", "accept"].as_slice(),
        &["ledger", "accept", "--preview"],
    ] {
        let fresh = vpt(&sandbox, args);
        assert_eq!(fresh.status.code(), Some(2), "{}", stderr(&fresh));
    }
    ingested(&sandbox, "a.m4a", FIRST);
    let before = snapshot(&sandbox);
    let healthy = vpt(&sandbox, &["ledger", "accept", "-o", "json"]);
    assert_eq!(healthy.status.code(), Some(2), "{}", stderr(&healthy));
    let document: serde_json::Value = serde_json::from_slice(&healthy.stderr).unwrap();
    assert!(
        document["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("nothing to accept"),
        "{document}"
    );
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn accept_help_says_it_refuses_when_there_is_nothing_to_accept() {
    let sandbox = machine("identity-accept-help");
    let help = stdout(&crate::succeeds(&sandbox, &["ledger", "accept", "--help"]));
    assert!(help.contains("nothing to accept"), "{help}");
    assert!(help.contains("vpt ledger accept --preview"), "{help}");
}

#[test]
fn a_restored_backup_is_written_to_after_accept() {
    let sandbox = machine("identity-restore");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    let refused = ingest(&sandbox, "c.m4a", FIRST + 120);
    assert_refused(&refused, "ledger_behind", &state(&sandbox));
    let accepted = crate::succeeds(&sandbox, &["ledger", "accept"]);
    assert!(accepted.stderr.is_empty(), "{}", stderr(&accepted));
    ingested(&sandbox, "c.m4a", FIRST + 120);
    assert_eq!(recordings(&sandbox), 3);
}
