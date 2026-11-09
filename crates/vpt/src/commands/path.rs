use crate::cli::output::Outcome;
use vpt_application::ports::{NoteLedger, NoteStage};
use vpt_domain::identity::RecordingId;
use vpt_protocol::{
    error::{ErrorDocument, ErrorKind},
    result::document,
};

pub(crate) fn run(ledger: &impl NoteLedger, id: &str, stage: &str) -> Outcome {
    let Ok(identity) = RecordingId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid recording id"));
    };
    let Some(stage) = NoteStage::parse(stage) else {
        return Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            "stage must be transcript or analysis",
        ));
    };
    match ledger.note_path(&identity, stage) {
        Ok(Some(path)) => Outcome::Success {
            completed: Vec::new(),
            diagnostics: Vec::new(),
            human: format!("{}\n", path.display()),
            document: document("path", serde_json::json!({"path": path})),
        },
        Ok(None) => Outcome::Failure(
            ErrorDocument::new(
                ErrorKind::Usage,
                "no pinned note path for this recording and stage",
            )
            .ids(vec![id.to_owned()]),
        ),
        Err(error) => Outcome::Failure(crate::compose::errors::ledger_error(error)),
    }
}
