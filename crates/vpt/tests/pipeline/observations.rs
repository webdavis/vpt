use super::*;

fn show(sandbox: &Sandbox, id: &str) -> serde_json::Value {
    json(&support::run(sandbox.vpt().args(["show", id, "--json"])))
}

#[test]
fn observations_expose_later_metadata_and_expiration_without_writes() {
    let (sandbox, row, artifacts) = expiration::prepared();
    let before = fixture::snapshot(&sandbox);
    let shown = show(&sandbox, row.id.as_str());
    assert_eq!(shown["engines"][0]["name"], "custom");
    assert_eq!(shown["engines"][0]["family"], "independent");
    assert_eq!(shown["language"], "en-US");
    assert_eq!(shown["requested_language"], "en-US");
    assert_eq!(shown["local"], true);
    assert_eq!(shown["diagnostics"], 1);
    assert_eq!(shown["open_flags"], 0);
    assert_eq!(shown["artifacts"].as_array().unwrap().len(), 8);
    for artifact in &artifacts {
        let observed = shown["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["path"] == artifact.path.to_str().unwrap())
            .unwrap();
        assert_eq!(observed["kind"], artifact.kind.as_str());
        assert_eq!(observed["digest"], artifact.digest.hex());
        assert_eq!(observed["status"], "published");
        assert_eq!(observed["trashed_at"], serde_json::Value::Null);
    }
    for (stage, kind) in [
        ("transcript", ArtifactKind::Transcript),
        ("analysis", ArtifactKind::Analysis),
    ] {
        let note = artifacts.iter().find(|a| a.kind == kind).unwrap();
        assert_eq!(shown["note_paths"][stage], note.path.to_str().unwrap());
    }
    let listed = json(&support::run(sandbox.vpt().args(["list", "--json"])));
    let mut body = shown.clone();
    body.as_object_mut().unwrap().remove("schema");
    body.as_object_mut().unwrap().remove("command");
    assert_eq!(listed["recordings"][0], body);
    assert!(!shown.to_string().contains("Private words"));
    assert_eq!(fixture::snapshot(&sandbox), before);

    expiration::age(artifacts.iter().map(|a| &a.path));
    json(&invoke(&sandbox, &[]));
    let before = fixture::snapshot(&sandbox);
    let expired = show(&sandbox, row.id.as_str());
    assert_eq!(expired["stages"]["note"], "expired");
    assert_eq!(expired["stages"]["analysis"], "expired");
    assert_eq!(expired["engines"], shown["engines"]);
    assert_eq!(expired["note_paths"], shown["note_paths"]);
    assert!(
        expired["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["status"] == "expired" && a["trashed_at"].is_string())
    );
    for stage in ["note", "analysis"] {
        let listed = json(&support::run(
            sandbox.vpt().args(["list", "--stage", stage, "--json"]),
        ));
        assert_eq!(listed["recordings"][0]["stages"][stage], "expired");
    }
    let storage = json(&support::run(sandbox.vpt().args(["storage", "--json"])));
    assert!(
        storage["stores"]
            .as_object()
            .unwrap()
            .values()
            .all(|s| s["files"] == 0)
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn observations_count_current_flags_and_hide_stale_verification() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    lexical_freshness::flag_first_word(&sandbox, &row);
    analysis::enable(&sandbox);
    json(&invoke(&sandbox, &[]));
    let before = fixture::snapshot(&sandbox);
    let observed = show(&sandbox, row.id.as_str());
    let review = json(&support::run(sandbox.vpt().args([
        "review",
        row.id.as_str(),
        "--json",
    ])));
    assert_eq!(observed["open_flags"], 2);
    assert_eq!(observed["open_flags"], review["open_flags"]);
    assert_eq!(observed["diagnostics"], review["diagnostic_count"]);
    assert_eq!(observed["flags"].as_array().unwrap().len(), 2);
    assert!(
        observed["flags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["artifact"] == "analysis")
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let flag = review["flags"][0]["identifier"].as_str().unwrap();
    json(&support::run_with_budget(
        sandbox.vpt().args([
            "review",
            row.id.as_str(),
            "--resolve",
            flag,
            "--confirm",
            "--json",
        ]),
        std::time::Duration::from_secs(5),
    ));
    let before = fixture::snapshot(&sandbox);
    let observed = show(&sandbox, row.id.as_str());
    assert_eq!(observed["open_flags"], 0);
    assert_eq!(observed["flags"], serde_json::json!([]));
    assert!(
        observed["stale_verification"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("analysis"))
    );
    assert_eq!(observed["stages"]["analysis"], "pending");
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn observations_report_pending_publications_without_recovering_them() {
    let (sandbox, row, artifacts) = expiration::prepared();
    let artifact = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Analysis)
        .unwrap();
    let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let writer = SqliteLedger::open(&root).unwrap();
    writer
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: artifact.path.clone(),
                expected_previous: None,
                intended: artifact.digest,
                recorded_at: row.ingested_at,
            }],
            ..Default::default()
        })
        .unwrap();
    drop(writer);
    let before = fixture::snapshot(&sandbox);
    let observed = show(&sandbox, row.id.as_str());
    let pending = observed["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["kind"] == "analysis")
        .unwrap();
    assert_eq!(pending["status"], "publication_pending");
    assert_eq!(fixture::snapshot(&sandbox), before);
}
