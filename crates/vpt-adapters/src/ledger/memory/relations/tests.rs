macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::relations::$name(&super::super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::relations::cases!(case);
