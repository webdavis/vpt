use super::{
    PathBuf, Verb, analyze, brief, confirm, examples, groups, import, redact, review, search, show,
};

#[derive(clap::Subcommand)]
pub(super) enum Command {
    /// Ingest, transcribe, publish and analyze every new recording.
    #[command(after_long_help = examples::RUN)]
    Run {
        /// Show what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
        /// Send actions the action command refused before.
        #[arg(long)]
        resend_refused: bool,
    },
    /// Choose the main engine and create the private directories.
    #[command(after_long_help = examples::SETUP)]
    Setup {
        /// Replace an existing configuration file.
        #[arg(long)]
        force: bool,
    },
    /// Check the configuration, stores, engines and helper.
    #[command(after_long_help = examples::DOCTOR)]
    Doctor,
    /// Archive new Voice Memos recordings.
    #[command(after_long_help = examples::INGEST)]
    Ingest {
        /// Show what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
        /// Archive only this one recording file.
        #[arg(long, value_name = "PATH")]
        once: Option<PathBuf>,
    },
    /// Archive one local audio or video file.
    #[command(after_long_help = examples::IMPORT)]
    Import(import::Options),
    /// Show a recording's saved state or one stage's text.
    #[command(after_long_help = examples::SHOW)]
    Show(show::Args),
    /// Transcribe one recording with the main engine.
    #[command(after_long_help = examples::TRANSCRIBE)]
    Transcribe {
        /// The recording id.
        id: String,
        /// Language tag for the engine, such as fr-CA.
        #[arg(long)]
        language: Option<String>,
        /// Show what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// List a transcript's open review flags, or resolve one.
    #[command(after_long_help = examples::REVIEW)]
    Review(review::Args),
    /// Confirm a term, or confirm or reject a recording's tag or relation.
    #[command(after_long_help = examples::CONFIRM)]
    Confirm(confirm::Args),
    /// Manage tags across every note vpt owns.
    #[command(subcommand, after_long_help = examples::DOCS)]
    Tag(groups::Tag),
    /// Analyze one recording, every recording, or ids read from stdin.
    #[command(after_long_help = examples::ANALYZE)]
    Analyze(analyze::Args),
    /// Print the saved path of a recording's stage or an occasion's brief.
    #[command(after_long_help = examples::PATH)]
    Path(groups::Path),
    /// Write notes from saved state.
    #[command(subcommand, after_long_help = examples::DOCS)]
    Note(groups::Note),
    /// Check that a note matches its recording.
    #[command(after_long_help = examples::VERIFY_NOTE)]
    VerifyNote(groups::VerifyNote),
    /// List saved occasions.
    #[command(after_long_help = examples::OCCASIONS)]
    Occasions,
    /// List recordings, optionally only those at one stage.
    #[command(after_long_help = examples::LIST)]
    List {
        /// Only recordings at this stage: transcribe, note or analysis.
        #[arg(long)]
        stage: Option<String>,
    },
    /// Search transcripts for words.
    #[command(after_long_help = examples::SEARCH)]
    Search(search::Options),
    /// Report file counts and sizes for each store.
    #[command(after_long_help = examples::STORAGE)]
    Storage,
    /// Remove files past their retention.
    #[command(subcommand, after_long_help = examples::DOCS)]
    Retention(groups::Retention),
    /// Write or preview a meeting brief.
    #[command(after_long_help = examples::BRIEF)]
    Brief(brief::Args),
    /// Release a redacted copy of a transcript or analysis.
    #[command(after_long_help = examples::REDACT)]
    Redact(redact::Options),
    /// Manage the home link.
    #[command(subcommand, after_long_help = examples::DOCS)]
    Symlink(groups::Symlink),
}

impl Command {
    pub(super) fn verb(self) -> Result<Verb, String> {
        Ok(match self {
            Self::Run {
                dry_run,
                resend_refused,
            } => Verb::Run {
                dry_run,
                resend_refused,
            },
            Self::Setup { force } => Verb::Setup { force },
            Self::Doctor => Verb::Doctor,
            Self::Ingest { dry_run, once } => Verb::Ingest { dry_run, once },
            Self::Import(options) => return options.verb(),
            Self::Show(args) => return args.verb(),
            Self::Transcribe {
                id,
                language,
                dry_run,
            } => Verb::Transcribe {
                id,
                language,
                dry_run,
            },
            Self::Review(args) => args.verb(),
            Self::Confirm(args) => args.verb(),
            Self::Tag(tag) => tag.verb(),
            Self::Analyze(args) => args.verb(),
            Self::Path(path) => path.verb(),
            Self::Note(note) => note.verb(),
            Self::VerifyNote(verify) => verify.verb(),
            Self::Occasions => Verb::Occasions,
            Self::List { stage } => Verb::List { stage },
            Self::Search(options) => Verb::Search(options),
            Self::Storage => Verb::Storage,
            Self::Retention(retention) => retention.verb(),
            Self::Brief(args) => return args.verb(),
            Self::Redact(options) => return options.verb(),
            Self::Symlink(symlink) => symlink.verb(),
        })
    }
}
