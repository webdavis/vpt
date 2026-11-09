macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::proposals::$name(&super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::proposals::cases!(case);
