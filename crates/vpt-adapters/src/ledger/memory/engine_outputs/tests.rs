macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::engine_outputs::$name(&super::super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::engine_outputs::cases!(case);
