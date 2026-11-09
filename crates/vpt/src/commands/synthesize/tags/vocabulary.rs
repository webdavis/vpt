use vpt_application::ports::{VocabularyKind, VocabularyPublication};
use vpt_domain::time::UtcInstant;
use vpt_protocol::error::ErrorDocument;

pub(super) fn prepare(
    path: &std::path::Path,
    learn: &[String],
    at: UtcInstant,
) -> Result<Option<VocabularyPublication>, Box<ErrorDocument>> {
    if learn.is_empty() {
        return Ok(None);
    }
    let update = vpt_adapters::known_tags::prepare_many(path, learn).map_err(|error| {
        crate::compose::publication::vocabulary::document(VocabularyKind::Tags, error)
    })?;
    Ok(update.added().then(|| VocabularyPublication {
        kind: VocabularyKind::Tags,
        entry: update.publication(at),
        bytes: update.content().to_vec(),
    }))
}
