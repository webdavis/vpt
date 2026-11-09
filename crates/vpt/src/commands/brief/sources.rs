use super::*;
use vpt_adapters::{known_terms, note_document::Document};
use vpt_application::ports::*;
use vpt_domain::brief::{Candidate, InputBudget, Term, TermTarget, TranscriptSource};

pub(super) fn collect(
    ledger: &impl BriefInputsLedger,
    settings: &Settings,
    roots: &Roots,
) -> Result<(Vec<Term>, Vec<Candidate>), Box<ErrorDocument>> {
    let index = crate::compose::index::read(settings, roots)?;
    let terms = known_terms::read(&settings.reconcile.known_terms_path)
        .map_err(crate::compose::publication::vocabulary::term_error)?;
    let terms = terms
        .into_iter()
        .map(|text| {
            let target = match index.matching(&text) {
                [] => TermTarget::Missing,
                [path] => TermTarget::Unique(path.clone()),
                _ => TermTarget::Ambiguous,
            };
            Term { text, target }
        })
        .collect();
    let stores = stores(roots)?;
    let mut candidates = Vec::new();
    let mut budget = InputBudget::default();
    let mut failed = None;
    let result = ledger.visit_brief_inputs(&mut |input| {
        let result = (|| {
            let source = source(&input, &stores, roots)?;
            let candidate = Candidate {
                recording: input.recording.id,
                captured_at: input.recording.captured_at,
                source,
                tags: input.tags,
                relations: input.relations,
            };
            budget.candidate(&candidate).map_err(|_| {
                Box::new(ErrorDocument::new(
                    ErrorKind::Ledger,
                    "brief inputs exceed capacity or are invalid",
                ))
            })?;
            candidates.push(candidate);
            Ok::<_, Box<ErrorDocument>>(())
        })();
        result.map_err(|error| {
            failed = Some(error);
            LedgerError::Corrupt("invalid brief input".into())
        })
    });
    if let Some(error) = failed {
        return Err(error);
    }
    result.map_err(ledger_error)?;
    Ok((terms, candidates))
}
fn source(
    input: &BriefInput,
    stores: &FilesystemStores,
    roots: &Roots,
) -> Result<Option<TranscriptSource>, Box<ErrorDocument>> {
    let (Some(note), Some(accepted)) = (&input.note, &input.review.accepted) else {
        return Ok(None);
    };
    if note.target.parent() != Some(roots.stores.transcripts.as_path()) {
        return Err(refused());
    }
    let current = match vpt_adapters::RootDir::open(&roots.stores.transcripts) {
        Ok(_) => stores
            .read_note(&note.target)
            .map_err(|error| Box::new(crate::compose::errors::store_error(error)))?,
        Err(vpt_adapters::ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => None,
        Err(error) => return Err(Box::new(crate::compose::errors::contained_error(error))),
    };
    let Some(current) = current else {
        return Ok(None);
    };
    let parsed = Document::parse(&current.text).map_err(|_| refused())?;
    parsed
        .check_owner(&input.recording.id, "transcript")
        .map_err(|_| refused())?;
    let content = parsed.content().map_err(|_| refused())?;
    if stores.digest_bytes(content.as_bytes()) != note.content_digest {
        return Err(refused());
    }
    Ok(Some(TranscriptSource {
        note: note.target.clone(),
        digest: current.digest,
        transcript: accepted.transcript.clone(),
        flags: input.review.flags.clone(),
        reviewed: accepted.lexical_classification
            && !input
                .review
                .stale_artifacts
                .contains(&NoteStage::Transcript),
    }))
}
fn refused() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(
            ErrorKind::Refused,
            "brief transcript source has unaccepted content or ownership",
        )
        .rule("brief_source"),
    )
}
