use crate::fixture::snapshot;
use crate::support::stdout;
use crate::{backup, deliver, ingested, machine, state};

const FIRST: i64 = 1_787_604_456;

#[test]
fn ledger_show_reports_the_set_and_this_machine_in_every_format() {
    let sandbox = machine("identity-show");
    ingested(&sandbox, "a.m4a", FIRST);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    let before = snapshot(&sandbox);
    let shown = crate::json(&sandbox, &["ledger", "show", "--json"]);
    assert_eq!(shown["command"], "ledger show");
    assert_eq!(shown["folder"], state(&sandbox).display().to_string());
    assert_eq!(shown["database"]["write_counter"], 2);
    assert_eq!(shown["database"]["recordings"], 2);
    assert_eq!(shown["database"]["id"].as_str().unwrap().len(), 32);
    assert_eq!(
        shown["this_machine"]["database_id"],
        shown["database"]["id"]
    );
    assert_eq!(shown["this_machine"]["write_counter"], 2);
    assert_eq!(shown["finding"], serde_json::Value::Null);
    let human = stdout(&crate::succeeds(&sandbox, &["ledger", "show"]));
    for words in [
        "folder: ",
        "database_id: ",
        "write_counter: 2",
        "recordings: 2",
    ] {
        assert!(human.contains(words), "{words}: {human}");
    }
    assert!(human.contains("this_machine: "), "{human}");
    let yaml = stdout(&crate::succeeds(
        &sandbox,
        &["ledger", "show", "--output", "yaml"],
    ));
    assert!(yaml.contains("write_counter: 2"), "{yaml}");
    crate::succeeds(&sandbox, &["ledger", "show", "--toon"]);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn ledger_show_names_the_finding_as_its_rule() {
    let sandbox = machine("identity-show-finding");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    let shown = crate::json(&sandbox, &["ledger", "show", "--json"]);
    assert_eq!(shown["finding"]["rule"], "ledger_behind");
    assert!(
        shown["finding"]["message"]
            .as_str()
            .unwrap()
            .ends_with("vpt ledger accept")
    );
    let human = stdout(&crate::succeeds(&sandbox, &["ledger", "show"]));
    assert!(human.contains("finding: ledger_behind"), "{human}");
}

#[test]
fn ledger_show_before_any_write_names_no_database_and_no_marker() {
    let sandbox = machine("identity-show-empty");
    let shown = crate::json(&sandbox, &["ledger", "show", "--json"]);
    assert_eq!(shown["database"], serde_json::Value::Null);
    assert_eq!(shown["this_machine"], serde_json::Value::Null);
    assert_eq!(shown["finding"], serde_json::Value::Null);
    assert!(!state(&sandbox).exists());
}

#[test]
fn bare_ledger_shows_help_and_exits_zero() {
    let sandbox = machine("identity-bare");
    let help = stdout(&crate::succeeds(&sandbox, &["ledger"]));
    assert!(help.contains("show"), "{help}");
    assert!(help.contains("accept"), "{help}");
    let help = stdout(&crate::succeeds(&sandbox, &["ledger", "show", "--help"]));
    assert!(help.contains("vpt ledger show --json"), "{help}");
}
