macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_tags_", stringify!($name)));
        }
    };
}
crate::ledger::contract::tags::cases!(case);
mod durability;

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if durability::run(scenario, root) {
        return;
    }
    let ledger = super::super::SqliteLedger::open(root).unwrap();
    macro_rules! run {
        ($name:ident) => {
            if scenario == concat!("ledger_tags_", stringify!($name)) {
                crate::ledger::contract::tags::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::tags::cases!(run);
    panic!("unknown tag scenario");
}
