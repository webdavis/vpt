macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::contract::flags::$name(&super::MemoryLedger::new());
        }
    };
}
crate::ledger::contract::flags::cases!(case);

#[test]
fn unchanged_identifiers_preserve_resolutions_in_memory() {
    use crate::ledger::contract::flags::{accepted, save, seed};
    use vpt_application::ports::{FlagLedger, HistoryWrite};
    use vpt_domain::{
        review::{FlagContent, Resolution},
        time::UtcInstant,
    };
    let ledger = super::MemoryLedger::new();
    let run = seed(&ledger);
    assert!(matches!(
        save(&ledger, run, accepted()),
        HistoryWrite::Committed(_)
    ));
    super::with(&ledger, |state| {
        for (_, flag) in &mut state.flags {
            if matches!(flag.content, FlagContent::Lexical(_)) {
                flag.resolution = Resolution::Confirmed {
                    at: UtcInstant { secs: 55 },
                };
            }
        }
    })
    .unwrap();
    let mut changed = accepted();
    changed.transcript.segments[0].words[0].confidence = Some(0.8);
    changed.uncertainty.lexical[0].confidence = Some(0.8);
    assert!(matches!(
        save(&ledger, run, changed),
        HistoryWrite::Committed(_)
    ));
    assert!(
        ledger
            .transcript_flags(&accepted().recording)
            .unwrap()
            .iter()
            .any(|flag| matches!(flag.content, FlagContent::Lexical(_))
                && matches!(flag.resolution, Resolution::Confirmed { .. }))
    );
}

mod verification {
    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                crate::ledger::contract::verification::$name(&super::super::MemoryLedger::new());
            }
        };
    }
    crate::ledger::contract::verification::cases!(case);
}
