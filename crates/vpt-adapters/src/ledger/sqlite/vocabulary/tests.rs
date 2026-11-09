macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_terms_", stringify!($name)));
        }
    };
}
crate::ledger::contract::known_terms::cases!(case);
mod durability;

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if durability::run(scenario, root) {
        return;
    }
    let ledger = super::SqliteLedger::open(root).unwrap();
    macro_rules! run {
        ($name:ident) => {
            if scenario == concat!("ledger_terms_", stringify!($name)) {
                crate::ledger::contract::known_terms::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::known_terms::cases!(run);
    panic!("unknown known-term scenario");
}
