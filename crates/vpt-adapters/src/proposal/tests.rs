use super::*;
use serde_json::json;
use vpt_domain::proposal::{Claim, Related, Tag};

#[test]
fn adapter_preserves_every_ordered_field_as_domain_data_and_hides_private_debug_text() {
    let document = json!({"schema":"vpt.proposal/1", "by":"Private author",
        "summary":[{"text":"Private first", "sources":["00:04-00:09", "invalid Private"]}, {"text":"Private second", "sources":[]}],
        "actions":[{"text":"Private action", "sources":["00:01-00:02"]}],
        "tags":[{"tag":"Private tag", "reason":"Private reason"}],
        "relations":[{"kind":"related", "target":"Private target", "reason":"Private related reason"}],
        "future":"Private additional value"
    });
    let decoded = decode(&serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(
        decoded.proposal,
        Proposal {
            by: "Private author".into(),
            summary: vec![
                Claim {
                    text: "Private first".into(),
                    source_references: vec!["00:04-00:09".into(), "invalid Private".into()]
                },
                Claim {
                    text: "Private second".into(),
                    source_references: vec![]
                }
            ],
            actions: vec![
                Claim {
                    text: "Private action".into(),
                    source_references: vec!["00:01-00:02".into()]
                }
                .into()
            ],
            tags: vec![Tag {
                tag: "Private tag".into(),
                reason: "Private reason".into()
            }],
            relations: vec![Related {
                target: "Private target".into(),
                reason: "Private related reason".into()
            }],
        }
    );
    assert_eq!(
        decoded.diagnostics,
        ["unknown additive proposal field: future"]
    );
    assert!(
        !format!(
            "{decoded:?} {:?} {:?} {:?}",
            decoded.proposal.summary, decoded.proposal.tags, decoded.proposal.relations
        )
        .contains("Private")
    );
}

#[test]
fn adapter_never_admits_invalid_or_oversized_wire_documents() {
    for bytes in [
        br#"{"schema":"vpt.proposal/2"}"#.to_vec(),
        b"Private malformed JSON".to_vec(),
        vec![b' '; 1_048_577],
    ] {
        let error = decode(&bytes).unwrap_err();
        assert!(!format!("{error:?}").contains("Private"));
    }
}
