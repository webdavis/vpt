use super::*;
use crate::ledger::{
    MemoryLedger,
    contract::{proposals::fixture, record},
};
use vpt_application::ports::{FlagLedger, TranscriptLedger};
use vpt_domain::{
    time::UtcInstant,
    uncertainty::{Class, Diagnostic, LexicalFlag},
};

fn input() -> (AcceptedTranscript, Vec<ReviewFlag>) {
    let ledger = MemoryLedger::new();
    fixture::seed(&ledger);
    (
        ledger.accepted_transcript(&record(81).id).unwrap().unwrap(),
        ledger.transcript_flags(&record(81).id).unwrap(),
    )
}

#[test]
fn flags_are_fingerprinted_in_identity_order_and_diagnostics_do_not_change_lexical_basis() {
    let (accepted, mut flags) = input();
    let content = LexicalFlag {
        class: Class::Other,
        ranges: std::iter::once(1..2).collect(),
        record_text: "🦆".into(),
        alternative_text: Some("bird".into()),
        confidence: None,
    };
    flags.push(ReviewFlag {
        identifier: crate::lexical_flag_id(&accepted.recording, &content),
        content: FlagContent::Lexical(content),
        resolution: Resolution::Open,
    });
    let expected = compute(&accepted, &flags).unwrap();
    flags.reverse();
    assert_eq!(compute(&accepted, &flags).unwrap(), expected);
    flags.push(ReviewFlag {
        identifier: crate::diagnostic_flag_id(&accepted.recording, Diagnostic::SingleEngine),
        content: FlagContent::Diagnostic(Diagnostic::SingleEngine),
        resolution: Resolution::Confirmed {
            at: UtcInstant { secs: 100 },
        },
    });
    assert_eq!(compute(&accepted, &flags).unwrap(), expected);
}

#[test]
fn corrections_resolutions_confidence_and_transcript_inputs_change_the_basis() {
    let (accepted, original) = input();
    let before = compute(&accepted, &original).unwrap();
    for resolution in [
        Resolution::Confirmed {
            at: UtcInstant { secs: 80 },
        },
        Resolution::Corrected {
            text: "Private correction".into(),
            at: UtcInstant { secs: 80 },
        },
        Resolution::Dismissed {
            at: UtcInstant { secs: 80 },
        },
    ] {
        let mut flags = original.clone();
        flags[0].resolution = resolution;
        let changed = compute(&accepted, &flags).unwrap();
        assert_eq!(changed.transcript, before.transcript);
        assert_ne!(changed.lexical_flags, before.lexical_flags);
    }
    let mut flags = original.clone();
    let FlagContent::Lexical(content) = &mut flags[0].content else {
        unreachable!()
    };
    content.confidence = Some(0.5);
    assert_ne!(
        compute(&accepted, &flags).unwrap().lexical_flags,
        before.lexical_flags
    );
    let mut changed = accepted.clone();
    changed.transcript.engine.version = "next".into();
    assert_ne!(
        compute(&changed, &original).unwrap().transcript,
        before.transcript
    );
    let mut repeated = accepted;
    repeated.accepted_at.secs += 10;
    assert_eq!(compute(&repeated, &original).unwrap(), before);
}
