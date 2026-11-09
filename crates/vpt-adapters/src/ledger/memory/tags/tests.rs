macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::tags::$name(&super::super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::tags::cases!(case);
