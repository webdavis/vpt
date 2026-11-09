macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::occasions::$name(&super::super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::occasions::cases!(case);
