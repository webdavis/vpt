use super::{PathBuf, Verb, text::PrivateText, values};
use vpt_domain::time::UtcInstant;

#[derive(Debug, PartialEq, Eq, clap::Args)]
pub(crate) struct Options {
    /// The audio or video file.
    pub file: PathBuf,
    /// The recording's title.
    #[arg(long, allow_hyphen_values = true)]
    pub title: Option<PrivateText>,
    /// When the recording was captured, as an RFC 3339 time.
    #[arg(long, value_parser = values::instant)]
    pub captured_at: Option<UtcInstant>,
}

impl Options {
    pub(super) fn verb(self) -> Result<Verb, String> {
        if self
            .title
            .as_ref()
            .is_some_and(|title| title.as_str().trim().is_empty() || title.as_str().contains('\0'))
        {
            return Err("invalid --title".into());
        }
        Ok(Verb::Import(self))
    }
}
