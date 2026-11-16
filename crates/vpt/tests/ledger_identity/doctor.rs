use crate::support::{Sandbox, stderr, stdout};
use crate::{backup, deliver, ingested, machine, point_at, state, vpt};
use serde_json::Value;

const FIRST: i64 = 1_787_604_456;

fn ledger_rows(sandbox: &Sandbox, code: i32) -> Vec<Value> {
    let output = vpt(sandbox, &["doctor", "ledger", "-o", "json"]);
    assert_eq!(output.status.code(), Some(code), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    let document: Value = serde_json::from_str(&stdout(&output)).unwrap();
    let checks = document["checks"].as_array().unwrap();
    let ledger = checks
        .iter()
        .find(|check| check["name"] == "ledger")
        .unwrap();
    ledger["rows"].as_array().unwrap().clone()
}

fn row(rows: &[Value], name: &str) -> Value {
    rows.iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no {name} row in {rows:?}"))
        .clone()
}

#[test]
fn doctor_shows_a_behind_ledger_as_an_issue_with_advice() {
    let sandbox = machine("identity-doctor-behind");
    ingested(&sandbox, "a.m4a", FIRST);
    assert_eq!(row(&ledger_rows(&sandbox, 0), "identity")["result"], "ok");
    let older = backup(&sandbox);
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    let identity = row(&ledger_rows(&sandbox, 1), "identity");
    assert_eq!(identity["result"], "issue");
    assert!(
        identity["detail"]
            .as_str()
            .unwrap()
            .starts_with("ledger_behind: "),
        "{identity}"
    );
    assert!(
        identity["advice"]
            .as_str()
            .unwrap()
            .contains("vpt ledger accept"),
        "{identity}"
    );
}

#[test]
fn doctor_warns_about_conflict_copies_beside_the_database() {
    let sandbox = machine("identity-doctor-copies");
    ingested(&sandbox, "a.m4a", FIRST);
    assert!(
        !ledger_rows(&sandbox, 0)
            .iter()
            .any(|row| row["name"] == "conflict_copies")
    );
    std::fs::copy(
        state(&sandbox).join("vpt.db"),
        state(&sandbox).join("vpt 2.db"),
    )
    .unwrap();
    let copies = row(&ledger_rows(&sandbox, 0), "conflict_copies");
    assert_eq!(copies["result"], "warning");
    assert!(
        copies["detail"].as_str().unwrap().contains("vpt 2.db"),
        "{copies}"
    );
}

#[test]
fn doctor_informs_when_the_database_folder_sits_in_a_synced_folder() {
    for (name, folder) in [
        (
            "icloud",
            "home/Library/Mobile Documents/com~apple~CloudDocs/vpt",
        ),
        ("cloud-storage", "home/Library/CloudStorage/SomeDrive/vpt"),
        ("dropbox", "home/Dropbox/vpt"),
    ] {
        let sandbox = machine(&format!("identity-doctor-synced-{name}"));
        point_at(&sandbox, &sandbox.path().join(folder));
        ingested(&sandbox, "a.m4a", FIRST);
        let synced = row(&ledger_rows(&sandbox, 0), "synced_folder");
        assert_eq!(synced["result"], "ok", "{synced}");
        let detail = synced["detail"].as_str().unwrap();
        assert!(detail.contains("sync service"), "{detail}");
        assert!(!detail.contains("move"), "{detail}");
    }
    let local = machine("identity-doctor-local");
    ingested(&local, "a.m4a", FIRST);
    assert!(
        !ledger_rows(&local, 0)
            .iter()
            .any(|row| row["name"] == "synced_folder")
    );
}
