macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::notes::$name(&super::super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::notes::cases!(case);
