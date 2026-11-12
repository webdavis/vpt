use super::*;
use crate::ledger::{MemoryLedger, contract::proposals::fixture};
use vpt_domain::{
    grounding::{Finding, GroundedClaim, SourceRange},
    proposal::{CheckedAction, CheckedClaim},
};
mod publication;

fn accepted() -> AcceptedProposal {
    let ledger = MemoryLedger::new();
    fixture::seed(&ledger);
    fixture::accepted(&ledger)
}

#[test]
fn versioned_saved_proposals_preserve_every_finding_shape_and_private_metadata() {
    let mut accepted = accepted();
    for (finding, sources, inherited, references) in [
        (
            None,
            vec![SourceRange {
                start_secs: 1,
                end_secs: 2,
            }],
            vec![],
            vec!["00:01-00:02".into()],
        ),
        (
            Some(Finding::Unsupported("PrivateMissing".into())),
            vec![SourceRange {
                start_secs: 0,
                end_secs: 1,
            }],
            vec![Sha256Digest([3; 32])],
            vec!["00:00-00:01".into()],
        ),
        (
            Some(Finding::BadReference),
            vec![],
            vec![],
            vec!["bad private reference".into()],
        ),
    ] {
        accepted.proposal.actions.push(CheckedAction {
            owner: None,
            due: None,
            claim: CheckedClaim {
                claim: vpt_domain::proposal::Claim {
                    text: "private claim".into(),
                    source_references: references,
                },
                grounding: GroundedClaim {
                    sources,
                    finding,
                    inherited_flags: inherited,
                },
            },
        });
    }
    let body = encode(&accepted).unwrap();
    assert_eq!(
        decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &body,
            accepted.accepted_at
        )
        .unwrap(),
        accepted
    );
    assert!(body.contains("PrivateMissing"));
    assert!(!format!("{accepted:?}").contains("PrivateMissing"));
}

#[test]
fn corrupt_versions_shapes_digests_and_citation_mappings_are_refused() {
    let accepted = accepted();
    let value: serde_json::Value = serde_json::from_str(&encode(&accepted).unwrap()).unwrap();
    for variant in 0..8 {
        let mut changed = value.clone();
        match variant {
            0 => changed["version"] = 999.into(),
            1 => changed["transcript_digest"] = "private-invalid-digest".into(),
            2 => changed["document"] = "private-invalid-document".into(),
            3 => changed["summary"] = serde_json::json!([]),
            4 => changed["summary"][0]["sources"] = serde_json::json!([[0, 2]]),
            5 => changed["summary"][0]["inherited_flags"] = serde_json::json!([]),
            6 => {
                changed["summary"][0]["inherited_flags"] =
                    serde_json::json!(["private-invalid-flag"])
            }
            _ => changed["unexpected-private"] = "private-extra-value".into(),
        }
        let result = decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &changed.to_string(),
            accepted.accepted_at,
        );
        assert_eq!(result, Err(invalid()), "variant {variant}");
        assert!(!format!("{result:?}").contains("private-"));
    }
}

#[test]
fn saved_input_and_grounding_expansion_are_bounded() {
    let mut accepted = accepted();
    assert_eq!(
        decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &" ".repeat(MAX_BODY + 1),
            accepted.accepted_at
        ),
        Err(invalid())
    );
    accepted.proposal.summary[0].claim.text = "a".repeat(65_537);
    assert_eq!(encode(&accepted), Err(invalid()));
    let mut accepted = self::accepted();
    accepted.proposal.summary[0].grounding.inherited_flags = vec![Sha256Digest([1; 32]); 65_537];
    assert_eq!(encode(&accepted), Err(invalid()));
}

#[test]
fn expanded_finding_text_is_refused_before_constructing_the_saved_document() {
    let mut accepted = accepted();
    accepted.proposal.summary[0].grounding.finding = Some(Finding::Unsupported("a".repeat(65_536)));
    accepted.proposal.summary = vec![accepted.proposal.summary[0].clone(); 257];
    assert_eq!(grounding::check(&accepted.proposal), Err(invalid()));
}

#[test]
fn action_owner_and_due_round_trip_and_their_saved_grounding_must_match() {
    use vpt_domain::proposal::{CheckedDue, Due, Owner};
    let mut accepted = accepted();
    accepted.proposal.actions[0].owner = Some(Owner::Other(Some("Private Dana".into())));
    accepted.proposal.actions[0].due = Some(CheckedDue {
        due: Due {
            date: "2026-11-19".into(),
            time: Some("15:00".into()),
            source_references: vec![],
        },
        grounding: GroundedClaim {
            sources: vec![],
            finding: Some(Finding::Unsourced),
            inherited_flags: vec![],
        },
    });
    let body = encode(&accepted).unwrap();
    let restored = decode(
        accepted.recording.clone(),
        &accepted.input_digest.hex(),
        &body,
        accepted.accepted_at,
    );
    assert_eq!(restored.unwrap(), accepted);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    let due = value["actions"][0]["due"].clone();
    for variant in 0..3 {
        let mut changed = value.clone();
        match variant {
            0 => {
                changed["actions"][0].as_object_mut().unwrap().remove("due");
            }
            1 => changed["summary"][0]["due"] = due.clone(),
            _ => changed["actions"][0]["due"]["finding"] = serde_json::Value::Null,
        }
        let result = decode(
            accepted.recording.clone(),
            &accepted.input_digest.hex(),
            &changed.to_string(),
            accepted.accepted_at,
        );
        assert_eq!(result, Err(invalid()), "variant {variant}");
    }
}
