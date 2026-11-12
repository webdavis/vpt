use super::*;
#[test]
fn broken_stdout_preserves_the_published_occasion_identity() {
    let sandbox = fixture::configured("brief-broken-output");
    let result = json(&invoke(&sandbox, &manual(&["--dry-run"])));
    let id = result["occasion"]["id"].as_str().unwrap();
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args(manual(&[])),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["completed"], serde_json::json!([id]));
    assert!(!stderr(&output).contains("Private"));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.occasions().unwrap()[0].briefed_at.is_some());
}
#[test]
fn interrupted_publication_retains_the_accepted_pack_for_startup_repair() {
    let sandbox = fixture::configured("brief-command-recovery");
    let first = json(&invoke(&sandbox, &manual(&[])));
    let id = first["occasion"]["id"].as_str().unwrap();
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection.execute_batch("CREATE TRIGGER refuse_brief_completion BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT,'Private interruption'); END").unwrap();
    fixture::preserve_wal(&connection);
    let output = invoke(&sandbox, &["brief", id, "--json"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private"));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    connection
        .execute_batch("DROP TRIGGER refuse_brief_completion")
        .unwrap();
    drop(connection);
    json(&invoke(&sandbox, &["brief", id, "--json"]));
    assert!(ledger.pending_publications().unwrap().is_empty());
    let artifact = ledger
        .brief(&vpt_domain::occasion::OccasionId::parse(id).unwrap())
        .unwrap()
        .unwrap();
    assert!(artifact.published_at.is_some());
    assert_eq!(
        std::fs::read_to_string(&artifact.target).unwrap(),
        artifact.text
    );
}
#[test]
fn obsidian_profile_rewrites_preserve_properties_and_pinned_path() {
    let sandbox = fixture::configured("brief-obsidian");
    let home = sandbox.path().join("home/.vpt");
    std::fs::create_dir_all(&home).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[note]\nprofile='obsidian'\nlink_style='wiki'\n[note.obsidian]\nvault_root={home:?}\nhub_briefs='Planning hub'\n"
        ),
    );
    let first = json(&invoke(&sandbox, &manual(&[])));
    let id =
        vpt_domain::occasion::OccasionId::parse(first["occasion"]["id"].as_str().unwrap()).unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let artifact = ledger.brief(&id).unwrap().unwrap();
    assert!(artifact.text.contains("[[Planning hub]]"));
    assert!(artifact.text.contains("aliases:"));
    assert!(artifact.text.contains("start-date:"));
    std::fs::write(
        &artifact.target,
        artifact
            .text
            .replace("vpt-schema:", "operator: keep\nvpt-schema:")
            + "\nOperator prose\n",
    )
    .unwrap();
    json(&invoke(&sandbox, &manual(&[])));
    let next = ledger.brief(&id).unwrap().unwrap();
    assert_eq!(next.target, artifact.target);
    assert_eq!(next.first_written, artifact.first_written);
    assert!(next.text.contains("operator: keep"));
    assert!(next.text.ends_with("\nOperator prose\n"));
}

#[test]
fn rescheduled_provider_occasion_rewrites_its_original_pinned_file() {
    use vpt_domain::{
        occasion::{OccasionDetails, OccasionKey, time::OccasionTime},
        run::{HistoryFact, RunKind},
        time::UtcInstant,
    };
    let sandbox = fixture::configured("brief-provider-pin");
    fixture::seed(&sandbox, vec![]);
    let ledger = SqliteLedger::open(
        &vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap(),
    )
    .unwrap();
    let key = OccasionKey::dam("Private provider identity").unwrap();
    let mut previous = None;
    for (title, at) in [
        ("Private original", "2026-10-01T09:00:00Z"),
        ("Private rescheduled", "2026-11-01T12:00:00-07:00"),
    ] {
        let now = UtcInstant { secs: 100 };
        let details = OccasionDetails {
            at: OccasionTime::parse(at).unwrap(),
            title: title.into(),
            duration_secs: Some(1800),
            participants: vec![],
            tags: vec![],
        };
        let change = vpt_adapters::prepare_occasion(&ledger, key.clone(), details, now).unwrap();
        let id = change.occasion.id.clone();
        let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Brief, now) else {
            panic!("run")
        };
        assert!(matches!(
            ledger.commit_step(
                run.run,
                &WorkDelta {
                    occasion: Some(change),
                    ..Default::default()
                },
                &[(
                    now,
                    HistoryFact::OccasionSaved {
                        occasion: id.clone()
                    }
                )]
            ),
            HistoryWrite::Committed(_)
        ));
        let result = json(&invoke(&sandbox, &["brief", id.as_str(), "--json"]));
        assert_eq!(result["occasion"]["at"], at);
        let artifact = ledger.brief(&id).unwrap().unwrap();
        assert_eq!(artifact.snapshot.occasion.title, title);
        if let Some((old_id, old_path, first_written)) = previous {
            assert_eq!(id, old_id);
            assert_eq!(artifact.target, old_path);
            assert_eq!(artifact.first_written, first_written);
        }
        previous = Some((id, artifact.target, artifact.first_written));
    }
    assert_eq!(
        std::fs::read_dir(sandbox.path().join("home/.vpt/briefs"))
            .unwrap()
            .count(),
        1
    );
}
