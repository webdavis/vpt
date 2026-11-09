macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_releases_", stringify!($name)));
        }
    };
}
mod faults;
mod migration;
crate::ledger::contract::releases::cases!(case);
pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if scenario == "ledger_releases_migration" {
        migration::run(root);
        return;
    }
    if scenario == "ledger_releases_ack_failure" {
        super::super::history::releases::tests::run(root);
        return;
    }
    if faults::run(scenario, root) {
        return;
    }
    let ledger = super::SqliteLedger::open(root).unwrap();
    macro_rules! run {
        ($name:ident) => {
            if scenario == concat!("ledger_releases_", stringify!($name)) {
                crate::ledger::contract::releases::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::releases::cases!(run);
    panic!("unknown release scenario");
}
