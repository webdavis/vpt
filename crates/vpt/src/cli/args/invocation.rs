use super::{Format, PathBuf, analyze, brief, import, redact, search, text::PrivateText};

#[derive(Debug, PartialEq, Eq)]
pub enum Verb {
    Print(String),
    Brief(brief::Options),
    Redact(redact::Options),
    Import(import::Options),
    Occasions,
    OccasionPath {
        id: String,
        stage: String,
    },
    Analyze {
        id: String,
        dry_run: bool,
        resend_refused: bool,
    },
    AnalyzeBatch {
        scope: analyze::Scope,
        dry_run: bool,
    },
    VerifyNote {
        path: PathBuf,
        recording: String,
    },
    Run {
        dry_run: bool,
        resend_refused: bool,
    },
    ReviewResolve {
        id: String,
        flag: String,
        action: vpt_domain::review::ReviewAction,
    },
    ConfirmTerm {
        term: PrivateText,
    },
    ConfirmRelation {
        id: String,
        relation: PrivateText,
    },
    ConfirmTag {
        id: String,
        tag: PrivateText,
        action: vpt_domain::tags::Action,
    },
    TagRename {
        old: PrivateText,
        new: PrivateText,
        dry_run: bool,
    },
    Version,
    Path {
        id: String,
        stage: String,
    },
    Setup {
        force: bool,
    },
    Doctor {
        checks: Vec<String>,
    },
    NoteWrite {
        id: String,
    },
    Transcribe {
        id: String,
        language: Option<String>,
        dry_run: bool,
    },
    Ingest {
        dry_run: bool,
        once: Option<PathBuf>,
    },
    Show {
        id: String,
        stage: Option<vpt_domain::copy::Kind>,
        redact: bool,
    },
    Search(search::Options),
    Review {
        id: String,
    },
    List {
        stage: Option<String>,
    },
    Storage,
    RetentionRun {
        dry_run: bool,
    },
    SymlinkDeploy,
    SymlinkVerify,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Invocation {
    pub verb: Verb,
    pub format: Option<Format>,
    pub config: Option<PathBuf>,
    pub progress: bool,
}

impl Invocation {
    pub(super) fn print(text: String) -> Self {
        Self {
            verb: Verb::Print(text),
            format: None,
            config: None,
            progress: false,
        }
    }
}
