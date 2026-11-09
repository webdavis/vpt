macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_occasions_", stringify!($name)));
        }
    };
}
crate::ledger::contract::occasions::cases!(case);

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if scenario == "ledger_occasions_corrupt" {
        super::corrupt::run(root);
        return;
    }
    if super::durability::run(scenario, root) {
        return;
    }
    let ledger = super::super::SqliteLedger::open(root).unwrap();
    macro_rules! run {
        ($name:ident) => {
            if scenario == concat!("ledger_occasions_", stringify!($name)) {
                crate::ledger::contract::occasions::$name(&ledger);
                return;
            }
        };
    }
    crate::ledger::contract::occasions::cases!(run);
    panic!("unknown tag scenario");
}
