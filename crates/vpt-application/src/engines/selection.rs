use super::{EngineConfig, EngineSettings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    InvalidLanguage,
    MissingEngine {
        slot: &'static str,
    },
    SameFamily {
        main: String,
        checker: String,
        family: String,
    },
}

#[derive(Debug)]
pub struct SelectedEngines<'a> {
    pub language: &'a str,
    pub main: &'a EngineConfig,
    pub checker: Option<&'a EngineConfig>,
}

impl EngineSettings {
    pub fn select<'a>(
        &'a self,
        language: Option<&'a str>,
    ) -> Result<SelectedEngines<'a>, SelectionError> {
        let language = language.unwrap_or(&self.language);
        if language.trim().is_empty() || language.chars().any(char::is_control) {
            return Err(SelectionError::InvalidLanguage);
        }
        let pair = self
            .by_language
            .iter()
            .find(|(tag, _)| tag.eq_ignore_ascii_case(language))
            .map_or(&self.pair, |(_, pair)| pair);
        let main = self
            .definitions
            .get(&pair.main)
            .ok_or(SelectionError::MissingEngine { slot: "main" })?;
        let checker = pair
            .checker
            .as_ref()
            .map(|name| {
                self.definitions
                    .get(name)
                    .ok_or(SelectionError::MissingEngine { slot: "checker" })
            })
            .transpose()?;
        if let Some(checker) = checker
            && main.family() == checker.family()
            && !self.allow_same_family
        {
            return Err(SelectionError::SameFamily {
                main: main.name.clone(),
                checker: checker.name.clone(),
                family: main.family().into(),
            });
        }
        Ok(SelectedEngines {
            language,
            main,
            checker,
        })
    }
}

#[cfg(test)]
mod tests;
