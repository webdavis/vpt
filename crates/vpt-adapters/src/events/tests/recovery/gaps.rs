use super::*;

#[test]
fn repeated_missing_retention_observation_counts_one_id_and_does_not_replay_work() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Recover, at(10))).run;
    let fact = HistoryFact::MissingRetentionHistory { intent_id: 7 };
    let saved = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[(at(11), fact.clone()), (at(12), fact.clone())],
    ));
    let script = fixture.path("report.sh");
    let capture = fixture.path("report");
    write_hook(&script);
    let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
    let before = fixture.history.recovery_view().unwrap();
    assert_eq!(
        sink(&fixture, &report, &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );

    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    let (evidence, window) = summary_fact(summaries(&after)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.affected_runs, vec![run]);
    assert_eq!(evidence.missing_history, 1);
    assert_eq!(evidence.completed_work, 0);
    assert_eq!(evidence.unknown_work, 0);
    assert_eq!(evidence.deferred_work, 0);
    assert_eq!(evidence.unresolved_effects, 0);
    assert_eq!(after.records[1].id, saved.records[0].id);
    assert_eq!(after.records[2].id, saved.records[1].id);
    assert_eq!(after.records[1].fact, fact);
    assert_eq!(after.records[2].fact, fact);
    assert!(!after.records.iter().any(|record| matches!(
        record.fact,
        HistoryFact::WorkStep { .. }
            | HistoryFact::EffectIntent { .. }
            | HistoryFact::EffectObserved { .. }
    )));
    assert!(!fixture.path("report.calls").exists());

    let documents = fixture.documents();
    assert_eq!(documents.len(), after.records.len());
    let observations: Vec<_> = documents
        .iter()
        .filter(|document| document["fact"]["tag"] == "missing_retention_history")
        .map(|document| {
            (
                document["event_id"].as_str().unwrap().to_owned(),
                document["fact"]["intent_id"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        observations,
        vec![
            (saved.records[0].id.render(), 7),
            (saved.records[1].id.render(), 7)
        ]
    );
    assert_eq!(
        sink(&fixture, &report, &[]).recover(&after),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fixture.history.recovery_view().unwrap(), after);
    assert_eq!(fixture.documents(), documents);
    assert!(!fixture.path("report.calls").exists());
}

#[test]
fn distinct_missing_retention_ids_survive_into_one_recovery_summary() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Recover, at(10))).run;
    let saved = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[
            (
                at(11),
                HistoryFact::MissingRetentionHistory { intent_id: 7 },
            ),
            (
                at(12),
                HistoryFact::MissingRetentionHistory { intent_id: 8 },
            ),
        ],
    ));
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );

    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    let (evidence, window) = summary_fact(summaries(&after)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.affected_runs, vec![run]);
    assert_eq!(evidence.missing_history, 2);
    assert_eq!(evidence.unresolved_effects, 0);
    assert_eq!(after.records[1].id, saved.records[0].id);
    assert_eq!(after.records[2].id, saved.records[1].id);
    assert!(
        !after
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::EffectIntent { .. }))
    );
    let documents = fixture.documents();
    let summary = documents
        .iter()
        .find(|document| document["fact"]["tag"] == "recovery_summary")
        .unwrap();
    assert_eq!(summary["fact"]["evidence"]["missing_history"], 2);
    let ids: Vec<_> = documents
        .iter()
        .filter(|document| document["fact"]["tag"] == "missing_retention_history")
        .map(|document| document["fact"]["intent_id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, vec![7, 8]);
}
