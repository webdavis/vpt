mod durability;
macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_briefs_", stringify!($name)));
        }
    };
}
crate::ledger::contract::briefs::cases!(case);

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if durability::run(scenario, root) {
        return;
    }
    let ledger = super::SqliteLedger::open(root).unwrap();
    macro_rules! run {
        ($name:ident) => {
            if scenario == concat!("ledger_briefs_", stringify!($name)) {
                crate::ledger::contract::briefs::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::briefs::cases!(run);
    panic!("unknown brief scenario");
}
