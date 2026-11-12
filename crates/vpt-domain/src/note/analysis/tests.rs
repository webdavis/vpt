use super::*;
mod resolutions;
mod safety;
use crate::{
    grounding::{Finding, Grounder},
    proposal::{Claim, Proposal, Related, Tag},
    transcript::Word,
};

fn claim(text: &str, sources: &[&str]) -> Claim {
    Claim {
        text: text.into(),
        source_references: sources.iter().map(|s| (*s).into()).collect(),
    }
}
fn proposal(summary: Vec<Claim>, actions: Vec<Claim>) -> Proposal {
    Proposal {
        by: "Private agent".into(),
        summary,
        actions: actions.into_iter().map(Into::into).collect(),
        tags: vec![Tag {
            tag: "private-tag".into(),
            reason: "private reason".into(),
        }],
        relations: vec![Related {
            target: "private-target".into(),
            reason: "private relation".into(),
        }],
    }
}
fn checked(proposal: Proposal) -> CheckedProposal {
    let words = [Word {
        text: "Alice 42".into(),
        start: 0.0,
        end: 2.0,
        confidence: None,
    }];
    Grounder::new(words.iter(), &[], 10.0, 0)
        .unwrap()
        .proposal(proposal)
        .unwrap()
}

#[test]
fn checks_each_claim_in_order_and_preserves_proposal_metadata() {
    let original = proposal(
        vec![claim("Alice", &["00:00-00:02"])],
        vec![claim("Bob", &["00:00-00:02"])],
    );
    let checked = checked(original.clone());
    assert_eq!(checked.by, original.by);
    assert_eq!(checked.tags, original.tags);
    assert_eq!(checked.relations, original.relations);
    assert_eq!(checked.summary.len(), 1);
    assert_eq!(checked.summary[0].claim, original.summary[0]);
    assert_eq!(checked.summary[0].grounding.finding, None);
    assert_eq!(checked.actions[0].claim.claim, original.actions[0].claim);
    assert_eq!(
        checked.actions[0].claim.grounding.finding,
        Some(Finding::Unsupported("Bob".into()))
    );
    for private in [
        "Private agent",
        "private-tag",
        "private reason",
        "private-target",
        "private relation",
        "Alice",
        "Bob",
    ] {
        assert!(!format!("{checked:?}").contains(private));
    }
}

#[test]
fn fixed_sections_preserve_order_citations_and_finding_annotations() {
    let checked = checked(proposal(
        vec![
            claim("Alice paid 42", &["00:00-00:02"]),
            claim("a paraphrase", &[]),
        ],
        vec![
            claim("Bob", &["00:00-00:02", "00:03-00:04"]),
            claim("a task", &["00:04-00:03"]),
        ],
    ));
    assert_eq!(
        analysis_content(&checked).unwrap(),
        "## Summary\n\n- Alice paid 42 [00:00-00:02]\n- a paraphrase [unsourced]\n\n## Actions\n\n- Bob [00:00-00:02] [00:03-00:04] [unsupported: Bob]\n- a task [00:04-00:03] [bad-reference]\n"
    );
    assert_eq!(
        analysis_content(&self::checked(proposal(vec![], vec![]))).unwrap(),
        "## Summary\n\n## Actions\n"
    );
}

#[test]
fn external_claims_and_invalid_citations_cannot_inject_markdown_or_managed_regions() {
    let checked = checked(proposal(
        vec![claim(
            "## Fake\n<!-- vpt:content:end --> [unsourced] **secret** https://private",
            &[],
        )],
        vec![claim("a task", &["bad]\n<script>secret</script>"])],
    ));
    let output = analysis_content(&checked).unwrap();
    assert!(output.contains("- &#35;&#35; Fake&#10;&lt;!-- vpt&#58;content&#58;end --&gt; &#91;unsourced&#93; &#42;&#42;secret&#42;&#42; https&#58;//private [unsourced]"));
    assert!(output.contains("[bad&#93;&#10;&lt;script&gt;secret&lt;/script&gt;] [bad-reference]"));
    assert!(!output.contains("<script>"));
    assert_eq!(output.lines().filter(|s| s.starts_with("## ")).count(), 2);
    assert_eq!(output.lines().filter(|s| s.starts_with("- ")).count(), 2);
}
