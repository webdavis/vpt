use super::{errors, prepare::Context};
mod vocabulary;
use vpt_application::ports::*;
use vpt_domain::{
    tags::{self, Options, Provenance, State, Tag},
    time::UtcInstant,
};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Changes {
    pub tags: Option<TagChange>,
    pub vocabulary: Option<VocabularyPublication>,
    pub rejections: Vec<tags::Rejected>,
}

pub(super) fn apply(
    input: &mut NoteInputs,
    proposal: &AcceptedProposal,
    context: &Context<'_>,
    at: UtcInstant,
) -> Result<Changes, Box<ErrorDocument>> {
    let (vocabulary, _) =
        crate::compose::tag_vocabulary::read(context.settings, context.mentions.index())?;
    let expected = input.tags.clone();
    let mut tags: Vec<_> = expected.as_ref().map_or_else(
        || {
            input
                .notes
                .iter()
                .filter(|note| note.stage == NoteStage::Transcript)
                .flat_map(|note| &note.managed_tags)
                .map(|name| Tag {
                    name: name.clone(),
                    state: State::Confirmed,
                    provenance: Provenance::Manual,
                    updated_at: None,
                })
                .collect()
        },
        |tags| {
            tags.iter()
                .filter(|tag| tag.updated_at.is_some() || tag.provenance == Provenance::Manual)
                .cloned()
                .collect()
        },
    );
    let preserved: std::collections::HashSet<_> = tags
        .iter()
        .map(|tag| tags::key(&tag.name))
        .collect::<Result<_, _>>()
        .map_err(|_| errors::invalid())?;
    let mut candidates = Vec::new();
    let mut indices = Vec::new();
    let mut rejections = Vec::new();
    let mut reasons = std::collections::HashMap::new();
    for (index, proposed) in proposal.proposal.tags.iter().enumerate() {
        let normalized = vocabulary
            .select(
                std::slice::from_ref(&proposed.tag),
                Options {
                    max_per_note: 1,
                    max_suggested: 1,
                    ..context.settings.tags.options
                },
            )
            .map_err(|_| errors::invalid())?;
        let Some(name) = normalized
            .confirmed
            .first()
            .or(normalized.suggested.first())
        else {
            rejections.push(tags::Rejected {
                index,
                reason: tags::Rejection::Syntax,
            });
            continue;
        };
        let key = tags::key(name).map_err(|_| errors::invalid())?;
        if preserved.contains(&key) {
            continue;
        }
        reasons.entry(key).or_insert(&proposed.reason);
        candidates.push(proposed.tag.clone());
        indices.push(index);
    }
    let options = context.settings.tags.options;
    let selection = vocabulary
        .select(
            &candidates,
            Options {
                max_per_note: options.max_per_note.saturating_sub(
                    tags.iter()
                        .filter(|tag| tag.state == State::Confirmed)
                        .count() as u32,
                ),
                max_suggested: options.max_suggested.saturating_sub(
                    tags.iter()
                        .filter(|tag| tag.state == State::Suggested)
                        .count() as u32,
                ),
                ..options
            },
        )
        .map_err(|_| errors::invalid())?;
    for (names, state) in [
        (&selection.confirmed, State::Confirmed),
        (&selection.suggested, State::Suggested),
    ] {
        for name in names {
            let key = tags::key(name).map_err(|_| errors::invalid())?;
            tags.push(Tag {
                name: name.clone(),
                state,
                updated_at: None,
                provenance: Provenance::Proposal {
                    by: proposal.proposal.by.clone(),
                    reason: (*reasons.get(&key).ok_or_else(errors::invalid)?).clone(),
                    input: proposal.input_digest,
                },
            });
        }
    }
    let change = (expected.as_ref() != Some(&tags)).then(|| TagChange {
        recording: input.record.id.clone(),
        expected,
        tags: tags.clone(),
        at,
    });
    input.tags = Some(tags);
    rejections.extend(selection.rejected.iter().map(|rejected| tags::Rejected {
        index: indices[rejected.index],
        reason: rejected.reason,
    }));
    let vocabulary =
        vocabulary::prepare(&context.settings.tags.known_tags_path, &selection.learn, at)?;
    Ok(Changes {
        tags: change,
        vocabulary,
        rejections,
    })
}
