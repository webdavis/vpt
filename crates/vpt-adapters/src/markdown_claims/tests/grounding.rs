use super::*;
use vpt_domain::{
    digest::Sha256Digest,
    grounding::Grounder,
    proposal::{Claim, Proposal},
    review::{FlagContent, Resolution, ReviewFlag},
    transcript::Word,
    uncertainty::{Class, LexicalFlag},
};

#[test]
fn generated_analysis_reverification_preserves_grounding_and_exact_annotation_bytes() {
    let words = [Word {
        text: "Alice".into(),
        start: 0.0,
        end: 1.0,
        confidence: Some(0.8),
    }];
    let flags = [ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            class: Class::AgreedUnverified,
            record_text: "Alice".into(),
            alternative_text: None,
            ranges: std::iter::once(0..1).collect(),
            confidence: Some(0.8),
        }),
    }];
    let proposal = Proposal {
        by: "private agent".into(),
        summary: vec![Claim {
            text: "Alice".into(),
            source_references: vec!["00:00-00:01".into()],
        }],
        actions: vec![
            Claim {
                text: "call Bob & ask".into(),
                source_references: vec!["00:00-00:01".into()],
            }
            .into(),
            Claim {
                text: "another action".into(),
                source_references: vec![],
            }
            .into(),
        ],
        tags: vec![],
        relations: vec![],
    };
    let checked = Grounder::new(words.iter(), &flags, 1.0, 0)
        .unwrap()
        .proposal(proposal)
        .unwrap();
    let rendered = vpt_domain::note::analysis_content(&checked).unwrap();
    let parsed = extract_content(&rendered).unwrap();
    assert_eq!(parsed.claims.len(), 3);
    let mut grounder = Grounder::new(words.iter(), &flags, 1.0, 0).unwrap();
    let mut findings = Vec::new();
    for (extracted, expected) in parsed.claims.iter().zip(checked.claims()) {
        assert_eq!(extracted.claim, expected.claim);
        let grounded = grounder.check(&extracted.claim).unwrap();
        assert_eq!(grounded, expected.grounding);
        findings.push(grounded.finding);
    }
    assert_eq!(parsed.annotate(&findings).unwrap(), rendered);
}

#[test]
fn content_scope_does_not_treat_a_managed_region_as_frontmatter() {
    let source = "---\n- action\n---\n";
    assert!(extract(source).unwrap().claims.is_empty());
    let parsed = extract_content(source).unwrap();
    assert_eq!(parsed.claims.len(), 1);
    assert_eq!(parsed.claims[0].claim.text, "action");
}
