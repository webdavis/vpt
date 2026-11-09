use std::fmt;

#[derive(Clone, PartialEq)]
pub struct Transcript {
    pub engine: EngineMetadata,
    pub language: String,
    pub language_source: LanguageSource,
    pub local: bool,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSource {
    Reported,
    Requested,
}

pub(crate) fn ends_sentence(text: &str) -> bool {
    text.trim_end_matches(['"', '\'', '”', '’', ')', ']', '}'])
        .ends_with(['.', '!', '?', '。', '！', '？'])
}

impl Transcript {
    pub fn language_mismatch(&self, requested: &str) -> bool {
        if self.language_source == LanguageSource::Requested {
            return false;
        }
        let reported_base = self.language.split('-').next().unwrap_or_default();
        let requested_base = requested.split('-').next().unwrap_or_default();
        if !reported_base.eq_ignore_ascii_case(requested_base) {
            return true;
        }
        self.language.contains('-')
            && requested.contains('-')
            && !self.language.eq_ignore_ascii_case(requested)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineMetadata {
    pub name: String,
    pub tool: String,
    pub version: String,
    pub model: String,
    pub family: String,
}

#[derive(Clone, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub words: Vec<Word>,
}

#[derive(Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
    pub confidence: Option<f64>,
}

impl fmt::Debug for Transcript {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Transcript")
            .field("engine", &self.engine)
            .field("language", &self.language)
            .field("local", &self.local)
            .field("segments", &self.segments.len())
            .finish()
    }
}

#[cfg(test)]
mod tests;
