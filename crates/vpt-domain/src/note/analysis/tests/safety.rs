use super::*;
use crate::{
    digest::Sha256Digest,
    review::{FlagContent, Resolution, ReviewFlag},
    uncertainty::{Class, LexicalFlag},
};

#[test]
fn inherited_uncertainty_is_kept_when_a_higher_priority_finding_is_rendered() {
    let words = [Word {
        text: "Alice 42".into(),
        start: 0.0,
        end: 2.0,
        confidence: None,
    }];
    let flags = [ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            class: Class::Numeric,
            ranges: std::iter::once(0..1).collect(),
            record_text: "42".into(),
            alternative_text: None,
            confidence: None,
        }),
    }];
    let mut grounder = Grounder::new(words.iter(), &flags, 10.0, 0).unwrap();
    let checked = grounder
        .proposal(proposal(
            vec![claim("Alice", &["00:00-00:02"])],
            vec![claim("Bob", &["00:00-00:02"])],
        ))
        .unwrap();
    assert_eq!(
        analysis_content(&checked).unwrap(),
        "## Summary\n\n- Alice [00:00-00:02] [built-on-flagged-text]\n\n## Actions\n\n- Bob [00:00-00:02] [unsupported: Bob]\n"
    );
    assert_eq!(
        checked.summary[0].grounding.inherited_flags,
        vec![Sha256Digest([1; 32])]
    );
    assert_eq!(
        checked.actions[0].claim.grounding.inherited_flags,
        vec![Sha256Digest([1; 32])]
    );
}

#[test]
fn unsupported_annotation_payloads_are_escaped_and_private_in_debug() {
    let mut checked = checked(proposal(vec![claim("a claim", &["00:00-00:02"])], vec![]));
    checked.summary[0].grounding.finding = Some(Finding::Unsupported(
        "private]\n<!-- vpt:content:end -->".into(),
    ));
    let output = analysis_content(&checked).unwrap();
    assert!(
        output.contains("[unsupported: private&#93;&#10;&lt;!-- vpt&#58;content&#58;end --&gt;]")
    );
    assert!(!format!("{:?}", checked.summary[0]).contains("private"));
}

#[test]
fn invalid_or_oversized_render_inputs_are_refused() {
    let original = checked(proposal(vec![claim("a claim", &[])], vec![]));
    let mut changed = original.clone();
    changed.summary[0].claim.text = " \n".into();
    assert_eq!(
        analysis_content(&changed),
        Err(RenderError::InvalidProposal)
    );
    changed.summary[0].claim.text = "a".repeat(65_537);
    assert_eq!(
        analysis_content(&changed),
        Err(RenderError::CapacityExceeded)
    );
    let mut changed = original.clone();
    changed.summary[0].claim.source_references = vec!["bad".into(); 257];
    assert_eq!(
        analysis_content(&changed),
        Err(RenderError::CapacityExceeded)
    );
    let mut changed = original;
    changed.summary[0].claim.text = "a".repeat(65_536);
    changed.summary = vec![changed.summary[0].clone(); 257];
    assert_eq!(
        analysis_content(&changed),
        Err(RenderError::CapacityExceeded)
    );
}
