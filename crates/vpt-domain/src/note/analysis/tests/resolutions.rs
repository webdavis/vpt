use super::*;
use crate::{review::Resolution, time::UtcInstant};

#[test]
fn finding_resolution_controls_the_visible_annotation_and_escapes_corrections() {
    let finding = Finding::Unsupported("Alice".into());
    let at = UtcInstant { secs: 42 };
    assert_eq!(
        resolved_finding_annotation(&finding, &Resolution::Open).unwrap(),
        Some("[unsupported: Alice]".into())
    );
    for resolution in [Resolution::Confirmed { at }, Resolution::Dismissed { at }] {
        assert_eq!(
            resolved_finding_annotation(&finding, &resolution).unwrap(),
            None
        );
    }
    let correction = Resolution::Corrected {
        text: "Bob]\n<!-- vpt:content end -->".into(),
        at,
    };
    assert_eq!(
        resolved_finding_annotation(&finding, &correction).unwrap(),
        Some("[corrected: Bob&#93;&#10;&lt;!-- vpt&#58;content end --&gt;]<!-- vpt:verification-correction -->".into())
    );
}

#[test]
fn invalid_correction_payloads_are_refused_before_rendering() {
    for text in [String::new(), " ".into(), "x".repeat(65_537)] {
        assert!(
            resolved_finding_annotation(
                &Finding::Unsourced,
                &Resolution::Corrected {
                    text,
                    at: UtcInstant { secs: 1 }
                }
            )
            .is_err()
        );
    }
}
