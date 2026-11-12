use super::prepare::Context;
use crate::compose::relation_target;
use vpt_application::ports::*;
use vpt_domain::relations::{Kind, Provenance, Relation, State};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn apply(
    input: &mut NoteInputs,
    proposal: &AcceptedProposal,
    context: &Context<'_>,
) -> Result<(), Box<ErrorDocument>> {
    input.relations.retain(|link| {
        link.relation.state != State::Suggested
            || !matches!(link.relation.provenance, Provenance::Proposal { .. })
    });
    let mut count = 0;
    for related in &proposal.proposal.relations {
        let (target, path) = match relation_target::resolve(
            &related.target,
            Kind::Related,
            input,
            context.ledger,
            context.mentions.index(),
        ) {
            Ok(value) => value,
            Err(error) if error.kind == ErrorKind::Usage => continue,
            Err(error) => return Err(error),
        };
        if input
            .relations
            .iter()
            .any(|link| link.relation.kind == Kind::Related && link.relation.target == target)
        {
            continue;
        }
        if count >= context.settings.relations.max_suggested {
            break;
        }
        input.relations.push(NoteRelation {
            path,
            relation: Relation {
                kind: Kind::Related,
                target,
                state: State::Suggested,
                provenance: Provenance::Proposal {
                    by: proposal.proposal.by.clone(),
                    reason: related.reason.clone(),
                    input: proposal.input_digest,
                },
                ranges: vec![],
            },
        });
        count += 1;
    }
    Ok(())
}
