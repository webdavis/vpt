use super::{TermsError, document};
use sha2::{Digest, Sha256};
use std::path::Component;
use vpt_application::ports::{VocabularyKind, VocabularyPublication};
use vpt_domain::digest::Sha256Digest;

pub(crate) fn validate(value: &VocabularyPublication) -> Result<(), TermsError> {
    let path = &value.entry.target;
    if !path.is_absolute()
        || path.file_name().is_none()
        || path
            .to_str()
            .is_none_or(|text| text.len() > 65_536 || text.chars().any(char::is_control))
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(TermsError::UnsafePath);
    }
    if document::lines(&value.bytes)?.is_empty()
        || Sha256Digest(Sha256::digest(&value.bytes).into()) != value.entry.intended
    {
        return Err(TermsError::InvalidText);
    }
    if value.kind == VocabularyKind::Tags {
        crate::known_tags::validate(&value.bytes)?;
    }
    Ok(())
}
