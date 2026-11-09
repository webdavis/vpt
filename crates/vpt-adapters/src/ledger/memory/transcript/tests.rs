macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::transcript::$name(&super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::transcript::cases!(case);
