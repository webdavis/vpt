use crate::fixture::{advice, configured, detail, result, run_json, seed_seen, snapshot};
use vpt_adapters::SCHEMA_VERSION;

#[test]
fn an_older_ledger_schema_warns_that_the_next_writing_command_upgrades_it() {
    let sandbox = configured("doctor-schema-older");
    seed_seen(&sandbox, false);
    let database = sandbox.path().join("state/vpt/vpt.db");
    rusqlite::Connection::open(&database)
        .unwrap()
        .pragma_update(None, "user_version", SCHEMA_VERSION - 1)
        .unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(result(&document, "ledger", "schema"), "warning");
    assert_eq!(
        detail(&document, "ledger", "schema"),
        format!("{}, older than {SCHEMA_VERSION}", SCHEMA_VERSION - 1)
    );
    assert_eq!(
        advice(&document, "ledger", "schema"),
        "the next writing command, such as `vpt run`, upgrades it"
    );
    assert_eq!(snapshot(&sandbox), before);
}
