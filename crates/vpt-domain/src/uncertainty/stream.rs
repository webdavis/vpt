use super::{Class, ClassificationError, LexicalFlag, Range, Transcript};
use crate::transcript::Word;
pub(super) use crate::transcript::ends_sentence;

pub(super) struct Stream<'a> {
    pub words: Vec<&'a Word>,
    pub text: Vec<&'a str>,
    pub sentence_initial: Vec<bool>,
}

impl<'a> Stream<'a> {
    pub fn new(transcript: &'a Transcript) -> Result<Self, ClassificationError> {
        let mut words = Vec::new();
        let mut bytes = 0usize;
        if transcript.segments.len() > 65_536 {
            return Err(ClassificationError::CapacityExceeded);
        }
        for word in transcript
            .segments
            .iter()
            .flat_map(|segment| &segment.words)
        {
            bytes = bytes.saturating_add(word.text.len());
            if words.len() == 65_536 || bytes > 16 * 1024 * 1024 {
                return Err(ClassificationError::CapacityExceeded);
            }
            if word
                .confidence
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            {
                return Err(ClassificationError::InvalidConfidence);
            }
            words.push(word);
        }
        let text = words.iter().map(|word| word.text.as_str()).collect();
        let mut initial = true;
        let sentence_initial = words
            .iter()
            .map(|word| {
                let before = initial;
                if word.text.chars().any(char::is_alphanumeric) {
                    initial = false;
                }
                if ends_sentence(&word.text) {
                    initial = true;
                }
                before
            })
            .collect();
        Ok(Self {
            words,
            text,
            sentence_initial,
        })
    }

    pub fn proper(&self, index: usize) -> bool {
        !self.sentence_initial[index]
            && self.text[index]
                .chars()
                .find(|ch| ch.is_alphabetic())
                .is_some_and(char::is_uppercase)
    }

    pub fn flag(
        &self,
        range: Range<usize>,
        class: Class,
        alternative_text: Option<String>,
    ) -> LexicalFlag {
        LexicalFlag {
            confidence: self.words[range.clone()]
                .iter()
                .filter_map(|word| word.confidence)
                .reduce(f64::min),
            record_text: self.text[range.clone()].join(" "),
            ranges: vec![range],
            class,
            alternative_text,
        }
    }
}
