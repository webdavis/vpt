use super::MemoryLedger;
use crate::ledger::contract;
use vpt_application::ports::{LedgerError, RetentionJournal};
use vpt_domain::time::UtcInstant;

impl contract::retention_history::RetentionPairFixture for MemoryLedger {
    fn seed_retention_pair(&self) -> contract::retention_history::PendingPair {
        contract::retention_history::pair_through_ports(self)
    }
}

mod recording_ledger_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                let ledger = MemoryLedger::new();
                contract::recording::$name(&ledger);
            }
        };
    }
    contract::recording_cases!(case);
}

mod publication_journal_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                let ledger = MemoryLedger::new();
                contract::journal::$name(&ledger);
            }
        };
    }
    contract::journal_cases!(case);
}

mod retention_journal_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                let ledger = MemoryLedger::new();
                contract::retention::$name(&ledger);
            }
        };
    }
    contract::retention_cases!(case);
}

#[test]
fn repeated_completion_keeps_the_first_recorded_time() {
    let ledger = MemoryLedger::new();
    let new = contract::retention::fixture(6);
    let id = ledger.record_intent(&new).expect("recorded");
    ledger
        .complete_intent(id, UtcInstant { secs: 8_000 })
        .expect("first");
    ledger
        .complete_intent(id, UtcInstant { secs: 8_001 })
        .expect("repeat");
    let state = ledger.state.lock().expect("state");
    assert_eq!(state.intents[0].1, Some(UtcInstant { secs: 8_000 }));
}

#[test]
fn intent_id_exhaustion_is_checked_without_inserting_a_row() {
    let ledger = MemoryLedger::new();
    ledger.state.lock().expect("state").next_intent_id = i64::MAX;
    let result = ledger.record_intent(&contract::retention::fixture(7));
    assert!(
        matches!(result, Err(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert!(ledger.state.lock().expect("state").intents.is_empty());
}

mod run_history_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                let ledger = MemoryLedger::new();
                contract::history::$name(&ledger);
            }
        };
    }
    contract::history_cases!(case);
}

mod retention_history_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                contract::retention_history::$name(&MemoryLedger::new());
            }
        };
    }
    contract::retention_history_cases!(case);
}
