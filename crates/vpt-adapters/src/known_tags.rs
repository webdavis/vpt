mod batch;
mod rename;
use crate::known_terms::{self, TermsError, Update};
pub use batch::{Edit, prepare_many};
pub use rename::prepare_renamed;
use std::path::Path;
use vpt_domain::tags::{NewTags, Options, TagError, Vocabulary};

pub fn read(path: &Path) -> Result<Vec<String>, TermsError> {
    let tags = known_terms::read(path)?;
    Vocabulary::new(&tags).map_err(error)?;
    Ok(tags)
}

pub fn prepare(path: &Path, tag: &str) -> Result<Update, TermsError> {
    if !vpt_domain::review::valid_correction(tag) {
        return Err(TermsError::InvalidText);
    }
    prepare_with_spelling(path, tag, false)
}

pub fn prepare_confirmed(path: &Path, tag: &str) -> Result<Update, TermsError> {
    vpt_domain::tags::key(tag).map_err(error)?;
    prepare_with_spelling(path, tag, true)
}

fn prepare_with_spelling(
    path: &Path,
    tag: &str,
    preserve_spelling: bool,
) -> Result<Update, TermsError> {
    known_terms::prepare_with(path, |bytes| {
        let tags: Vec<_> = known_terms::document::lines(bytes)?
            .into_iter()
            .map(str::to_owned)
            .collect();
        let vocabulary = Vocabulary::new(&tags).map_err(error)?;
        let selected = vocabulary
            .select(
                &[tag.to_owned()],
                Options {
                    new_tags: NewTags::Write,
                    max_per_note: 1,
                    max_suggested: 0,
                },
            )
            .map_err(error)?;
        let normalized = selected.confirmed.first().ok_or(TermsError::InvalidText)?;
        let spelling = if preserve_spelling && !selected.learn.is_empty() {
            tag
        } else {
            normalized
        };
        known_terms::document::append(bytes, spelling)
    })
}

fn error(error: TagError) -> TermsError {
    match error {
        TagError::CapacityExceeded => TermsError::TooLarge,
        TagError::InvalidVocabulary => TermsError::InvalidText,
    }
}

pub(crate) fn validate(bytes: &[u8]) -> Result<(), TermsError> {
    let tags: Vec<_> = known_terms::document::lines(bytes)?
        .into_iter()
        .map(str::to_owned)
        .collect();
    Vocabulary::new(&tags).map(|_| ()).map_err(error)
}

#[cfg(test)]
mod tests;
