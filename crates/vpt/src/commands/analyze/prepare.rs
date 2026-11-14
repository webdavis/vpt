use super::errors;
use crate::{commands::note, compose::errors::ledger_error};
use vpt_adapters::{
    FilesystemStores, Roots, SystemClock, note_document::NoteFormatter, verification,
};
use vpt_application::{Settings, ports::*};
use vpt_domain::{grounding::Grounder, review::FlagContent, time::UtcInstant};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Context<'a> {
    pub settings: &'a Settings,
    pub roots: &'a Roots,
    pub stores: &'a FilesystemStores,
    pub ledger: &'a vpt_adapters::SqliteLedger,
    pub mentions: &'a crate::compose::mentions::Context,
}

pub(super) struct Prepared {
    pub delta: WorkDelta,
    pub rejections: Vec<vpt_domain::tags::Rejected>,
    pub renamed: Vec<usize>,
}
pub(super) fn delta(
    mut input: NoteInputs,
    output: AnalysisOutput,
    context: &Context<'_>,
    at: UtcInstant,
) -> Result<Prepared, Box<ErrorDocument>> {
    let settings = context.settings;
    let roots = context.roots;
    let stores = context.stores;
    let transcript = input.review.accepted.as_ref().ok_or_else(errors::invalid)?;
    let basis = vpt_adapters::grounding_basis_for(transcript, &input.review.flags)
        .map_err(|error| Box::new(ledger_error(error)))?;
    let proposal = Grounder::for_transcript(
        &transcript.transcript,
        &input.review.flags,
        transcript.duration,
        settings.analysis.grounding_window_secs,
    )
    .map_err(|_| errors::invalid())?
    .proposal(output.proposal)
    .map_err(|_| errors::invalid())?;
    let content = vpt_domain::note::analysis_content(&proposal).map_err(|_| errors::invalid())?;
    let previous = input.review.flags_for_recheck();
    let checked = verification::check(
        &content,
        Some(NoteStage::Analysis),
        transcript,
        &previous,
        settings.analysis.grounding_window_secs,
    )
    .map_err(|_| errors::invalid())?;
    let findings = verification::review_flags(
        transcript,
        NoteStage::Analysis,
        &checked.findings,
        &previous,
    )
    .map_err(|_| errors::invalid())?;
    input.review.flags.retain(|flag| !matches!(&flag.content, FlagContent::Verification(finding) if finding.artifact == NoteStage::Analysis));
    input.review.flags.extend(findings);
    input
        .review
        .verified_claims
        .retain(|claims| claims.artifact != NoteStage::Analysis);
    input
        .review
        .stale_artifacts
        .retain(|stage| *stage != NoteStage::Analysis);
    input.review.stale_findings.retain(|flag| !matches!(&flag.content, FlagContent::Verification(finding) if finding.artifact == NoteStage::Analysis));
    let mut accepted = AcceptedProposal {
        vocabulary_publication: None,
        recording: input.record.id.clone(),
        input_digest: output.input_digest,
        basis,
        grounding_window_secs: settings.analysis.grounding_window_secs,
        proposal,
        accepted_at: at,
    };
    let expected_relations: Vec<_> = input
        .relations
        .iter()
        .map(|link| link.relation.clone())
        .collect();
    let tags = super::tags::apply(&mut input, &accepted, context, at)?;
    super::relations::apply(&mut input, &accepted, context)?;
    let selected_relations: Vec<_> = input
        .relations
        .iter()
        .map(|link| link.relation.clone())
        .collect();
    let mut delta = vpt_application::note::prepare(
        &input,
        settings,
        vpt_application::note::WriteOptions {
            roots: &roots.stores,
            analysis_content: Some(&checked.content),
            regenerate_expired: false,
        },
        at,
        SystemClock.offset_at(at),
        stores,
        &NoteFormatter,
    )
    .map_err(note::errors::document)?;
    let (relations, ambiguities) = delta
        .relations
        .take()
        .map_or((selected_relations, None), |change| {
            (change.relations, change.ambiguities)
        });
    delta.relations =
        (relations != expected_relations || ambiguities.is_some()).then(|| RelationChange {
            recording: input.record.id.clone(),
            expected: expected_relations,
            relations,
            ambiguities,
            at,
        });
    delta.tags = tags.tags;
    delta.vocabulary = tags.vocabulary;
    delta.learned_tags = tags.learned;
    delta.actions = crate::compose::actions::queue(
        settings,
        &input.record.id,
        input.proposal.as_ref().map(|previous| &previous.proposal),
        &accepted.proposal,
    );
    accepted.vocabulary_publication = delta.vocabulary.as_ref().map(|value| value.entry.clone());
    delta.proposal = Some(ProposalChange::Accept(Box::new(accepted)));
    Ok(Prepared {
        delta,
        rejections: tags.rejections,
        renamed: tags.renamed,
    })
}
