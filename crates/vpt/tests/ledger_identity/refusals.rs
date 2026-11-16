use crate::fixture::snapshot;
use crate::support;
use crate::{assert_refused, backup, deliver, ingest, ingested, machine, record, state, vpt};

const FIRST: i64 = 1_787_604_456;

fn refuses_and_writes_nothing(sandbox: &support::Sandbox, rule: &str) {
    record(sandbox, "waiting.m4a", FIRST + 900);
    let before = snapshot(sandbox);
    let output = vpt(sandbox, &["ingest", "-o", "json"]);
    assert_eq!(snapshot(sandbox), before);
    let document: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(document["error"]["rule"], rule);
    assert_eq!(document["error"]["kind"], "refused");
    let output = vpt(sandbox, &["ingest"]);
    assert_refused(&output, rule, &state(sandbox));
    assert_eq!(snapshot(sandbox), before);
}

#[test]
fn an_older_copy_of_the_ledger_is_refused_as_behind() {
    let sandbox = machine("identity-behind");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    refuses_and_writes_nothing(&sandbox, "ledger_behind");
    let text = support::stderr(&vpt(&sandbox, &["ingest"]));
    assert!(
        text.contains("this ledger is older than the one this machine last wrote"),
        "{text}"
    );
}

#[test]
fn a_ledger_from_another_machine_is_refused_as_replaced() {
    let mine = machine("identity-replaced-mine");
    let theirs = machine("identity-replaced-theirs");
    ingested(&mine, "a.m4a", FIRST);
    ingested(&theirs, "b.m4a", FIRST + 60);
    deliver(&state(&theirs).join("vpt.db"), &state(&mine));
    refuses_and_writes_nothing(&mine, "ledger_replaced");
}

#[test]
fn a_ledger_that_went_missing_is_refused_as_replaced() {
    let sandbox = machine("identity-missing");
    ingested(&sandbox, "a.m4a", FIRST);
    std::fs::remove_file(state(&sandbox).join("vpt.db")).unwrap();
    refuses_and_writes_nothing(&sandbox, "ledger_replaced");
    assert!(!state(&sandbox).join("vpt.db").exists());
}

#[test]
fn a_ledger_another_machine_wrote_from_an_older_base_is_refused_as_diverged() {
    let mine = machine("identity-diverged-mine");
    let theirs = machine("identity-diverged-theirs");
    ingested(&mine, "a.m4a", FIRST);
    deliver(&state(&mine).join("vpt.db"), &state(&theirs));
    ingested(&theirs, "b.m4a", FIRST + 60);
    ingested(&mine, "c.m4a", FIRST + 120);
    deliver(&state(&theirs).join("vpt.db"), &state(&mine));
    refuses_and_writes_nothing(&mine, "ledger_diverged");
    let text = support::stderr(&ingest(&mine, "d.m4a", FIRST + 180));
    assert!(text.contains("this machine's last write is gone"), "{text}");
}
