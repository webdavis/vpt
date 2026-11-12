use std::collections::HashSet;
use vpt_application::ports::{LearnedTagChange, TagLedger, VocabularyKind, VocabularyPublication};
use vpt_domain::time::UtcInstant;
use vpt_protocol::error::ErrorDocument;

pub(super) fn prepare(
    path: &std::path::Path,
    learn: &[String],
    drop: &HashSet<String>,
    ledger: &impl TagLedger,
    at: UtcInstant,
) -> Result<(Option<VocabularyPublication>, Option<LearnedTagChange>), Box<ErrorDocument>> {
    if learn.is_empty() && drop.is_empty() {
        return Ok((None, None));
    }
    let edit = vpt_adapters::known_tags::prepare_many(path, learn, drop).map_err(|error| {
        crate::compose::publication::vocabulary::document(VocabularyKind::Tags, error)
    })?;
    let forgotten: Vec<_> = ledger
        .learned_tags()
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?
        .into_iter()
        .filter(|key| !edit.keys.contains(key))
        .collect();
    let learned = (!edit.appended.is_empty() || !forgotten.is_empty()).then(|| LearnedTagChange {
        learned: edit.appended.clone(),
        forgotten,
    });
    let publication = edit.update.added().then(|| VocabularyPublication {
        kind: VocabularyKind::Tags,
        entry: edit.update.publication(at),
        bytes: edit.update.content().to_vec(),
    });
    Ok((publication, learned))
}
