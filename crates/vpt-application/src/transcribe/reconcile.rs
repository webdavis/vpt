use super::Transcription;
use vpt_domain::uncertainty::{self, Classification, ClassificationError, Diagnostic, Options};

impl Transcription {
    pub fn classify(
        &self,
        options: Options,
        known_terms: &[&str],
    ) -> Result<Classification, ClassificationError> {
        let mut result = if self.lexical_classification {
            uncertainty::classify(
                &self.record.transcript,
                self.comparison().map(|output| &output.transcript),
                options,
                known_terms,
            )?
        } else {
            Classification::default()
        };
        if self.single_engine() {
            result.diagnostics.push(Diagnostic::SingleEngine);
        }
        if std::iter::once(&self.record)
            .chain(self.checker.as_ref())
            .any(|output| output.transcript.language_mismatch(&self.language))
        {
            result.diagnostics.push(Diagnostic::LanguageMismatch);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
