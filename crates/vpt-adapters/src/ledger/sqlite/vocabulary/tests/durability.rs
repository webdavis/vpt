use crate::ledger::contract::{engine_outputs::committed, known_terms::*};
use crate::ledger::sqlite::{SqliteLedger, map};
use vpt_application::ports::*;
use vpt_domain::{run::RunKind, time::UtcInstant};

#[test]
fn pending_known_terms_survive_observational_reopening() {
    crate::ledger::sqlite::tests::supervised("ledger_terms_reopen");
}
#[test]
fn pending_known_tags_survive_observational_reopening() {
    crate::ledger::sqlite::tests::supervised("ledger_terms_reopen_tags");
}
macro_rules! corrupt_case {
    ($name:ident, $index:literal) => {
        #[test]
        fn $name() {
            crate::ledger::sqlite::tests::supervised_prepared(
                concat!("ledger_terms_corrupt_", $index),
                |root| {
                    let ledger = seed(root);
                    corrupt(&ledger, $index);
                },
            );
        }
    };
}
corrupt_case!(oversized_term_body_is_refused_before_loading, 0);
corrupt_case!(wrong_typed_term_body_is_refused_privately, 1);
corrupt_case!(changed_term_body_is_refused_privately, 2);
corrupt_case!(invalid_previous_digest_is_refused_privately, 3);
corrupt_case!(invalid_intended_digest_is_refused_privately, 4);
corrupt_case!(invalid_publication_time_is_refused_privately, 5);
corrupt_case!(invalid_vocabulary_kind_is_refused_privately, 6);
corrupt_case!(oversized_vocabulary_kind_is_refused_before_loading, 7);

fn seed(root: &crate::RootDir) -> SqliteLedger {
    let ledger = SqliteLedger::open(root).unwrap();
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    committed(ledger.commit_step(run, &delta(), &facts()));
    ledger
}

fn corrupt(ledger: &SqliteLedger, index: usize) {
    let updates = [
        "UPDATE known_term_publications SET body=zeroblob(16777217)",
        "UPDATE known_term_publications SET body='private canary'",
        "UPDATE known_term_publications SET body=X'416E6E0A'",
        "UPDATE dirty_publications SET expected_previous=zeroblob(65537)",
        "UPDATE dirty_publications SET intended='private digest canary'",
        "UPDATE dirty_publications SET recorded_at='private time canary'",
        "UPDATE known_term_publications SET kind='private canary'",
        "UPDATE known_term_publications SET kind=zeroblob(16777217)",
    ];
    ledger
        .transaction(|connection| {
            connection
                .execute_batch("PRAGMA ignore_check_constraints=ON")
                .map_err(map)?;
            connection.execute(updates[index], []).map_err(map)?;
            Ok(())
        })
        .unwrap();
}

pub(super) fn run(scenario: &str, root: &crate::RootDir) -> bool {
    if scenario == "ledger_terms_reopen_tags" {
        let ledger = SqliteLedger::open(root).unwrap();
        let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
        let tags = tag_publication();
        committed(ledger.commit_step(
            run,
            &WorkDelta {
                vocabulary: Some(tags.clone()),
                ..Default::default()
            },
            &tag_facts(),
        ));
        drop(ledger);
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(
            observer.vocabulary_publication(&tags.entry.target).unwrap(),
            Some(tags)
        );
        assert_eq!(observer.pending_publications().unwrap().len(), 1);
        return true;
    }
    if scenario == "ledger_terms_reopen" {
        drop(seed(root));
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(
            observer
                .vocabulary_publication(&publication().entry.target)
                .unwrap(),
            Some(publication())
        );
        assert_eq!(
            observer.pending_publications().unwrap(),
            vec![publication().entry]
        );
        assert_eq!(observer.pending_exports().unwrap().len(), 2);
        return true;
    }
    if !scenario.starts_with("ledger_terms_corrupt_") {
        return false;
    }
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let error = observer
        .vocabulary_publication(&publication().entry.target)
        .unwrap_err();
    assert_eq!(error, crate::ledger::vocabulary::invalid());
    assert!(!format!("{error:?}").contains("private"));
    true
}
