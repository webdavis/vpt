macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_verification_", stringify!($name)));
        }
    };
}
crate::ledger::contract::verification::cases!(case);

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if let Some(case) = scenario.strip_prefix("ledger_verification_basis_") {
        basis::run(case, root);
        return;
    }
    if scenario == "ledger_verification_reopen" {
        durability::reopen(root);
        return;
    }
    if let Some(case) = scenario.strip_prefix("ledger_verification_corrupt_") {
        corruption::run(case, root);
        return;
    }
    let ledger = super::super::SqliteLedger::open(root).unwrap();
    macro_rules! dispatch {
        ($name:ident) => {
            if scenario == concat!("ledger_verification_", stringify!($name)) {
                crate::ledger::contract::verification::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::verification::cases!(dispatch);
    panic!("unknown verification scenario");
}

mod basis;
mod corruption;
mod durability;
#[test]
fn verification_survives_reopen_and_read_only_observation() {
    super::super::tests::supervised("ledger_verification_reopen");
}
