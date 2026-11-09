use serde::{
    Serialize, Serializer,
    ser::{Error, SerializeStruct},
};
use std::path::Path;
use vpt_domain::{
    brief::{Certainty, Selector},
    identity::RecordingId,
};

pub struct Source<'a> {
    pub recording: &'a RecordingId,
    pub at_secs: f64,
    pub note: &'a Path,
}
pub struct Item<'a> {
    pub text: &'a str,
    pub certainty: Option<Certainty>,
    pub sources: &'a [Source<'a>],
    pub selector: Selector,
}
impl Serialize for Source<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if !self.at_secs.is_finite() || self.at_secs < 0.0 || self.at_secs > (1u64 << 53) as f64 {
            return Err(S::Error::custom("invalid brief source"));
        }
        let path = path(self.note).ok_or_else(|| S::Error::custom("invalid brief source"))?;
        let mut state = serializer.serialize_struct("Source", 3)?;
        state.serialize_field("recording", self.recording.as_str())?;
        state.serialize_field("at_secs", &self.at_secs)?;
        state.serialize_field("note", path)?;
        state.end()
    }
}
impl Serialize for Item<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let certainty = self
            .certainty
            .ok_or_else(|| S::Error::custom("brief item requires certainty"))?;
        if self.sources.is_empty()
            || self.sources.len() > 256
            || self.text.trim().is_empty()
            || self.text.len() > 4 * 65_536
            || self.text.chars().count() > 65_536
            || self.text.contains('\0')
        {
            return Err(S::Error::custom("invalid brief item"));
        }
        let mut state = serializer.serialize_struct("Item", 4)?;
        state.serialize_field("text", self.text)?;
        state.serialize_field("certainty", certainty.as_str())?;
        state.serialize_field("sources", self.sources)?;
        state.serialize_field("selector", self.selector.as_str())?;
        state.end()
    }
}
use vpt_domain::brief::document::source_path as path;
