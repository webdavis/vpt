use crate::ledger::contract::{engine_outputs::committed, notes::*};
use crate::{RootDir, SqliteLedger};
use vpt_application::ports::*;

#[test]
fn notes_survive_reopening_and_observational_reads() {
    super::super::super::tests::supervised("ledger_notes_reopen");
}
#[test]
fn failed_history_append_rolls_back_note_path_bytes_and_publication() {
    super::super::super::tests::supervised("ledger_notes_rollback");
}
macro_rules! corrupt_case {
    ($name:ident, $index:literal) => {
        #[test]
        fn $name() {
            if matches!($index, "3" | "4") {
                super::super::super::tests::supervised_prepared(
                    concat!("ledger_notes_prepared_", $index),
                    |root| {
                        let ledger = SqliteLedger::open(root).unwrap();
                        let run = seed(&ledger);
                        committed(ledger.commit_step(run, &delta(), &facts()));
                        corrupt(&ledger, $index.parse().unwrap());
                    },
                );
            } else {
                super::super::super::tests::supervised(concat!("ledger_notes_corrupt_", $index));
            }
        }
    };
}
corrupt_case!(changed_note_body_is_refused_privately, "0");
corrupt_case!(invalid_content_digest_is_refused_privately, "1");
corrupt_case!(foreign_managed_tag_is_refused_privately, "2");
corrupt_case!(oversized_note_body_is_refused_before_loading, "3");
corrupt_case!(oversized_tags_are_refused_before_loading, "4");
corrupt_case!(relative_note_pin_is_refused, "5");
corrupt_case!(oversized_note_pin_is_refused_before_loading, "6");

pub(super) fn run(scenario: &str, root: &RootDir) -> bool {
    if let Some(index) = scenario.strip_prefix("ledger_notes_prepared_") {
        let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        inspect(&ledger, index.parse().unwrap());
        return true;
    }
    if !matches!(scenario, "ledger_notes_reopen" | "ledger_notes_rollback")
        && !scenario.starts_with("ledger_notes_corrupt_")
    {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let run = seed(&ledger);
    if scenario == "ledger_notes_rollback" {
        ledger.transaction(|connection| connection.execute_batch("CREATE TRIGGER refuse_note_history BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT, 'fixture'); END;").map_err(crate::ledger::sqlite::map)).unwrap();
        assert!(matches!(
            ledger.commit_step(run, &delta(), &facts()),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(ledger.note_at(&artifact().target).unwrap(), None);
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
        ledger
            .read(|connection| {
                let pin: Option<String> = connection
                    .query_row("SELECT transcript_note_path FROM recordings", [], |row| {
                        row.get(0)
                    })
                    .unwrap();
                assert_eq!(pin, None);
                Ok(())
            })
            .unwrap();
        return true;
    }
    committed(ledger.commit_step(run, &delta(), &facts()));
    if scenario == "ledger_notes_reopen" {
        drop(ledger);
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        let note = artifact();
        assert_eq!(observer.note_at(&note.target).unwrap(), Some(note.clone()));
        assert_eq!(
            observer.note(&note.recording, note.stage).unwrap(),
            Some(note)
        );
        assert_eq!(
            observer.pending_publications().unwrap(),
            delta().commit.publications
        );
        return true;
    }
    let index: usize = scenario
        .strip_prefix("ledger_notes_corrupt_")
        .unwrap()
        .parse()
        .unwrap();
    corrupt(&ledger, index);
    inspect(&ledger, index);
    true
}

fn corrupt(ledger: &SqliteLedger, index: usize) {
    let assignments = [
        "body = 'private corrupted note'",
        "content_digest = 'bad'",
        "managed_tags = '[\"private-corrupt-tag\"]'",
        "body = zeroblob(67108865)",
        "managed_tags = zeroblob(16777217)",
        "target = 'relative.md'",
        "target = zeroblob(65537)",
    ];
    let assignment = assignments[index];
    ledger
        .transaction(|connection| {
            connection
                .execute_batch("PRAGMA ignore_check_constraints = ON")
                .map_err(crate::ledger::sqlite::map)?;
            connection
                .execute(&format!("UPDATE owned_notes SET {assignment}"), [])
                .map_err(crate::ledger::sqlite::map)?;
            Ok(())
        })
        .unwrap();
}

fn inspect(ledger: &SqliteLedger, index: usize) {
    if index < 5 {
        assert_eq!(
            ledger
                .note_path(&artifact().recording, artifact().stage)
                .unwrap(),
            Some(artifact().target)
        );
    } else {
        assert!(matches!(
            ledger.note_path(&artifact().recording, artifact().stage),
            Err(LedgerError::Corrupt(_))
        ));
    }
    let error = ledger
        .note(&artifact().recording, artifact().stage)
        .unwrap_err();
    assert!(matches!(error, LedgerError::Corrupt(_)));
    assert!(!format!("{error:?}").contains("private"));
}
