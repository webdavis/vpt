use super::{PathBuf, Verb, text::PrivateText};
use vpt_domain::{identity::RecordingId, note::NoteStage};

#[derive(Debug, PartialEq, Eq, clap::Args)]
pub(crate) struct Options {
    /// The recording id.
    #[arg(value_parser = recording_id)]
    pub id: RecordingId,
    /// The stage to release: transcript or analysis.
    #[arg(long, default_value = "transcript", value_parser = stage)]
    pub stage: NoteStage,
    /// Write the released copy into this directory.
    #[arg(long = "to", value_name = "DIR")]
    pub destination: Option<PathBuf>,
    /// The released copy's title.
    #[arg(long, allow_hyphen_values = true)]
    pub title: Option<PrivateText>,
}

fn recording_id(text: &str) -> Result<RecordingId, String> {
    RecordingId::parse(text).map_err(|_| "invalid recording id".into())
}

fn stage(text: &str) -> Result<NoteStage, String> {
    NoteStage::parse(text).ok_or_else(|| "must be transcript or analysis".into())
}

impl Options {
    pub(super) fn verb(self) -> Result<Verb, String> {
        if self.title.as_ref().is_some_and(|t| {
            t.as_str().trim().is_empty() || t.as_str().len() > 65_536 || t.as_str().contains('\0')
        }) {
            return Err("invalid --title".into());
        }
        Ok(Verb::Redact(self))
    }
}
