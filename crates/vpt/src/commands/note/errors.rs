use vpt_application::{note::NoteWriteError, ports::NoteFormatError};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn not_found() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, "recording not found"))
}

pub(crate) fn document(error: NoteWriteError) -> Box<ErrorDocument> {
    let refused = |rule, message| ErrorDocument::new(ErrorKind::Refused, message).rule(rule);
    Box::new(match error {
        NoteWriteError::NoTranscript => {
            ErrorDocument::new(ErrorKind::Usage, "recording has no accepted transcript")
        }
        NoteWriteError::AnalysisUnavailable => refused(
            "analysis_unavailable",
            "accepted analysis render inputs are unavailable".into(),
        ),
        NoteWriteError::Ambiguous(paths) => refused(
            "ambiguous_note",
            format!(
                "multiple notes claim this recording: {}",
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
        NoteWriteError::Foreign(path) => refused(
            "foreign_note",
            format!("unowned note occupies {}", path.display()),
        ),
        NoteWriteError::Store(error) => crate::compose::errors::store_error(error),
        NoteWriteError::Render(_) => {
            ErrorDocument::new(ErrorKind::Ledger, "saved note cannot be rendered")
        }
        NoteWriteError::Format { path, error } => {
            let rule = match error {
                NoteFormatError::Markers => "markers",
                NoteFormatError::UnsupportedSchema(_) => "schema_ahead",
                NoteFormatError::Identity | NoteFormatError::Stage => "foreign_note",
                NoteFormatError::Frontmatter => "frontmatter",
                NoteFormatError::CapacityExceeded => "note_limit",
            };
            refused(rule, format!("note rewrite refused at {}", path.display()))
        }
    })
}
