use crate::ports::{Prompt, PromptError};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SetupError {
    NoTerminal,
    Exists(PathBuf),
    PathEscape(PathBuf),
    NonPrivateConfig(PathBuf),
    NonPrivateDirectory(PathBuf),
    MultiplyLinkedConfig(PathBuf),
    Io(String),
}

pub trait SetupWriter {
    fn config_path(&self) -> PathBuf;
    fn config_exists(&self) -> bool;
    fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError>;
    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError>;
}

#[derive(Debug)]
pub struct SetupOutcome {
    pub written: PathBuf,
    pub main_engine: String,
}

pub struct Setup<R: Fn(&str) -> String> {
    pub engines: Vec<String>,
    pub render: R,
    pub directories: Vec<PathBuf>,
}

impl<R: Fn(&str) -> String> Setup<R> {
    pub fn run<P: Prompt, W: SetupWriter>(
        &self,
        prompt: &mut P,
        writer: &W,
        force: bool,
    ) -> Result<SetupOutcome, SetupError> {
        if writer.config_exists() && !force {
            return Err(SetupError::Exists(writer.config_path()));
        }
        let options: Vec<_> = self.engines.iter().map(String::as_str).collect();
        let chosen = prompt
            .choose("Main transcription engine", &options, 0)
            .map_err(|error| match error {
                PromptError::NoTerminal => SetupError::NoTerminal,
                PromptError::Closed => {
                    SetupError::Io("the terminal closed before an answer".into())
                }
            })?;
        let main_engine = self
            .engines
            .get(chosen)
            .ok_or_else(|| SetupError::Io("invalid engine choice".into()))?
            .clone();
        let written = writer.write_config(&(self.render)(&main_engine), force)?;
        for directory in &self.directories {
            writer.create_private_dir(directory)?;
        }
        Ok(SetupOutcome {
            written,
            main_engine,
        })
    }
}

#[cfg(test)]
mod tests;
