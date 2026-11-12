use super::action_ids;
use crate::{
    grounding::{GroundedClaim, SourceRange},
    identity::RecordingId,
    proposal::{CheckedAction, CheckedClaim, Claim},
};

fn action(text: &str, start: Option<u64>) -> CheckedAction {
    CheckedAction {
        claim: CheckedClaim {
            claim: Claim {
                text: text.into(),
                source_references: vec![],
            },
            grounding: GroundedClaim {
                sources: start
                    .map(|start_secs| SourceRange {
                        start_secs,
                        end_secs: start_secs + 1,
                    })
                    .into_iter()
                    .collect(),
                finding: None,
                inherited_flags: vec![],
            },
        },
        owner: None,
        due: None,
    }
}

#[test]
fn actions_sharing_a_second_or_lacking_a_source_get_ordered_ids_that_ignore_wording_even_as_twins()
{
    let id = RecordingId::parse("2026-08-24T144736-0123456789ab").unwrap();
    let ids = |first: &str| {
        action_ids(
            &id,
            &[
                action(first, Some(40)),
                action(first, Some(40)),
                action("third", None),
                action("fourth", Some(3_725)),
                action("fifth", None),
            ],
        )
    };
    let expected = [
        "2026-08-24T144736-0123456789ab@00:40",
        "2026-08-24T144736-0123456789ab@00:40#2",
        "2026-08-24T144736-0123456789ab#1",
        "2026-08-24T144736-0123456789ab@62:05",
        "2026-08-24T144736-0123456789ab#2",
    ];
    assert_eq!(ids("first"), expected);
    assert_eq!(ids("reworded entirely"), expected);
}
