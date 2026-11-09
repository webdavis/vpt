mod apply;
mod candidates;
mod matches;
mod model;
mod patterns;
mod vocabulary;
pub use candidates::{Candidate, MaskedText, candidates};
pub use model::*;
const MAX_TEXT: usize = 16 * 1024 * 1024;
const MAX_ITEMS: usize = 65_536;

pub struct Redactor {
    terms: vocabulary::Vocabulary,
    names: vocabulary::Vocabulary,
    options: Options,
    mappings: Vec<Mapping>,
    mapped_bytes: usize,
}
impl Redactor {
    pub fn new(terms: &[&str], names: &[&str], options: Options) -> Result<Self, Error> {
        model::validate_mask(&options.mask)?;
        if options.classes.len() > 7 {
            return Err(Error::Capacity);
        }
        Ok(Self {
            terms: vocabulary::Vocabulary::new(terms, Class::Term)?,
            names: vocabulary::Vocabulary::new(names, Class::Note)?,
            options,
            mappings: vec![],
            mapped_bytes: 0,
        })
    }
    pub fn apply(&mut self, text: &str) -> Result<String, Error> {
        self.apply_protected(text, &[])
    }
    pub fn apply_protected(
        &mut self,
        text: &str,
        protected: &[std::ops::Range<usize>],
    ) -> Result<String, Error> {
        self.apply_with_spans(text, protected)
            .map(|result| result.text)
    }
    pub fn mappings(&self) -> &[Mapping] {
        &self.mappings
    }
    pub fn verify(&self, text: &str) -> Result<(), Error> {
        validate_text(text)?;
        let tokens = vocabulary::tokens(text)?;
        let values: Vec<_> = self.mappings.iter().map(|m| m.value.as_str()).collect();
        let matcher = vocabulary::Vocabulary::new(&values, Class::Term)?;
        if let Some(result) = matcher.first(&tokens) {
            let mapping = &self.mappings[result?];
            return Err(Error::Residue {
                class: mapping.class,
                number: Some(mapping.number),
            });
        }
        let mut found = Vec::new();
        self.terms.scan(&tokens, &mut found)?;
        self.names.scan(&tokens, &mut found)?;
        if let Some(found) = found.first() {
            return Err(Error::Residue {
                class: found.class,
                number: None,
            });
        }
        Ok(())
    }
}
fn validate_text(text: &str) -> Result<(), Error> {
    if text.len() > MAX_TEXT {
        return Err(Error::Capacity);
    }
    if text.contains('\0') {
        return Err(Error::InvalidText);
    }
    Ok(())
}
fn append(output: &mut String, text: &str) -> Result<(), Error> {
    if text.len() > MAX_TEXT - output.len() {
        return Err(Error::Capacity);
    }
    output.push_str(text);
    Ok(())
}
#[cfg(test)]
mod tests;
