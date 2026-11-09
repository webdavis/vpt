mod bounded;
mod encode;
mod item;
mod selection;
mod validate;
pub use item::{Item, Source};
#[cfg(test)]
use vpt_domain::{
    brief::ContextSummary,
    occasion::{OccasionDetails, OccasionId, OccasionSource},
    time::UtcInstant,
};

pub const SCHEMA: &str = "vpt.brief/1";
pub use vpt_domain::brief::document::{Document, DocumentError as EncodeError};

pub fn encode(document: &Document<'_>) -> Result<String, EncodeError> {
    bounded::encode(&encode::value(document)?)
}

#[cfg(test)]
mod tests;
