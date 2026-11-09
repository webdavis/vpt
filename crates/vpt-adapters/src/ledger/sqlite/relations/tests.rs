mod ambiguities;
mod durability;
macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_relations_", stringify!($name)));
        }
    };
}
crate::ledger::contract::relations::cases!(case);
pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if ambiguities::run(scenario, root) || durability::run(scenario, root) {
        return;
    }
    let ledger = super::super::SqliteLedger::open(root).unwrap();
    macro_rules! run {
        ($name:ident) => {
            if scenario == concat!("ledger_relations_", stringify!($name)) {
                crate::ledger::contract::relations::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::relations::cases!(run);
    panic!("unknown relation scenario");
}
