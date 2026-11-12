use super::*;
use serde_json::{Value, json};
mod bounds;

fn document() -> Value {
    json!({"schema":"vpt.proposal/1", "by":"Private author",
        "summary":[{"text":"Private first", "sources":["04:12-04:19", "02:07-02:11"]}, {"text":"Private second", "sources":[]}],
        "actions":[{"text":"Private action", "sources":["00:00-00:02"]}],
        "tags":[{"tag":"Private tag", "reason":"Private reason"}],
        "relations":[{"kind":"related", "target":"Private target", "reason":"Private relation reason"}]
    })
}
fn decoded(value: &Value) -> Result<DecodedProposal, ProposalDecodeError> {
    decode(&serde_json::to_vec(value).unwrap())
}

#[test]
fn preserves_proposal_order_text_and_citations_without_exposing_them_in_debug() {
    let result = decoded(&document()).unwrap();
    assert_eq!(result.document.by, "Private author");
    assert_eq!(
        result.document.summary[0].sources,
        ["04:12-04:19", "02:07-02:11"]
    );
    assert_eq!(result.document.summary[1].text, "Private second");
    assert_eq!(serde_json::to_value(&result.document).unwrap(), document());
    assert!(
        !format!(
            "{result:?} {:?} {:?} {:?}",
            result.document.summary, result.document.tags, result.document.relations
        )
        .contains("Private")
    );
}

#[test]
fn unknown_major_versions_are_named_and_noncanonical_schemas_are_refused() {
    let mut value = document();
    value["schema"] = json!("vpt.proposal/2");
    assert_eq!(
        decoded(&value),
        Err(ProposalDecodeError::UnsupportedVersion(2))
    );
    for schema in [
        "vpt.proposal/01",
        "vpt.proposal/+1",
        "vpt.proposal/Private",
        "vpt.engine/1",
    ] {
        value["schema"] = json!(schema);
        assert_eq!(decoded(&value), Err(ProposalDecodeError::Invalid("schema")));
    }
}

#[test]
fn preserves_ungrounded_claims_and_invalid_tag_syntax_for_the_product_gates() {
    let mut value = document();
    value["summary"][0]["sources"] = json!(["bad-reference", "00:09-00:01", "999:00-999:01"]);
    value["tags"][0]["tag"] = json!("#123");
    let result = decoded(&value).unwrap();
    assert_eq!(result.document.summary[0].sources[0], "bad-reference");
    assert!(result.document.summary[1].sources.is_empty());
    assert_eq!(result.document.tags[0].tag, "#123");
}

#[test]
fn unknown_additive_fields_are_named_without_their_values() {
    let mut value = document();
    value["future"] = json!("Private value");
    value["summary"][0]["future"] = json!("Private value");
    value["actions"][0]["future"] = json!("Private value");
    value["tags"][0]["future"] = json!("Private value");
    value["relations"][0]["future"] = json!("Private value");
    let decoded = decoded(&value).unwrap();
    assert_eq!(
        decoded.diagnostics,
        [
            "unknown additive proposal field: actions.future",
            "unknown additive proposal field: future",
            "unknown additive proposal field: relations.future",
            "unknown additive proposal field: summary.future",
            "unknown additive proposal field: tags.future"
        ]
    );
    assert!(!format!("{decoded:?}").contains("Private value"));
}

#[test]
fn malformed_documents_and_non_related_proposals_are_refused_without_private_values() {
    for (pointer, value) in [
        ("/by", json!("")),
        ("/by", json!("Private\u{0}")),
        ("/summary/0/text", json!(" ")),
        ("/actions/0/text", json!("Private\u{7}")),
        ("/summary/0/sources", json!(null)),
        ("/summary/0/sources/0", json!(42)),
        ("/tags/0/reason", json!(true)),
        ("/tags/0/tag", json!("Private\u{0}")),
        ("/relations/0/target", json!("")),
        ("/relations/0/kind", json!("supersedes")),
        ("/relations/0/kind", json!("continues")),
        ("/relations/0/kind", json!("mentions")),
        ("/relations/0/kind", json!("Private-new-kind")),
    ] {
        let mut doc = document();
        *doc.pointer_mut(pointer).unwrap() = value;
        let error = decoded(&doc).unwrap_err();
        assert!(!format!("{error:?}").contains("Private"));
    }
    for field in ["by", "summary", "actions", "tags", "relations"] {
        let mut doc = document();
        doc.as_object_mut().unwrap().remove(field);
        assert_eq!(decoded(&doc), Err(ProposalDecodeError::Invalid("document")));
    }
    let mut bytes = serde_json::to_vec(&document()).unwrap();
    bytes.extend_from_slice(b" {} Private");
    assert!(matches!(
        decode(&bytes),
        Err(ProposalDecodeError::Limit(LimitViolation::Syntax(_)))
    ));
}

#[test]
fn an_empty_proposal_is_valid_and_claim_markup_remains_literal_data() {
    let empty = json!({"schema":"vpt.proposal/1", "by":"agent", "summary":[], "actions":[], "tags":[], "relations":[]});
    assert!(decoded(&empty).is_ok());
    let mut doc = document();
    let text = "# Private heading\n<!-- vpt:content end --> [unverified] [link](private)";
    doc["summary"][0]["text"] = json!(text);
    assert_eq!(decoded(&doc).unwrap().document.summary[0].text, text);
}

#[test]
fn an_action_owner_round_trips_and_a_blank_name_is_refused() {
    let mut value = document();
    for owner in [
        json!("me"),
        json!({"other": "Dana"}),
        json!({"other": null}),
    ] {
        value["actions"][0]["owner"] = owner;
        let result = decoded(&value).unwrap();
        assert_eq!(serde_json::to_value(&result.document).unwrap(), value);
        assert!(result.diagnostics.is_empty());
    }
    value["actions"][0]["owner"] = json!(null);
    let result = decoded(&value).unwrap();
    assert_eq!(serde_json::to_value(&result.document).unwrap(), document());
    for owner in [json!({"other": " "}), json!("them"), json!({"other": 1})] {
        value["actions"][0]["owner"] = owner;
        assert!(decoded(&value).is_err());
    }
}
