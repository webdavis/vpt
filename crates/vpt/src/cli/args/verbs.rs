use super::{PathBuf, Verb, analyze, brief, confirm, groups, import, redact, review, search, show};

#[derive(clap::Subcommand)]
pub(super) enum Command {
    /// Ingest, transcribe, publish and analyze every new recording.
    Run {
        /// Show what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
        /// Send actions the action command refused before.
        #[arg(long)]
        resend_refused: bool,
    },
    /// Choose the main engine and create the private directories.
    Setup {
        /// Replace an existing configuration file.
        #[arg(long)]
        force: bool,
    },
    /// Check the configuration, stores, engines and helper.
    Doctor,
    /// Archive new Voice Memos recordings.
    Ingest {
        /// Show what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
        /// Archive only this one recording file.
        #[arg(long, value_name = "PATH")]
        once: Option<PathBuf>,
    },
    /// Archive one local audio or video file.
    Import(import::Options),
    /// Show a recording's saved state or one stage's text.
    Show(show::Args),
    /// Transcribe one recording with the main engine.
    Transcribe {
        id: String,
        /// Language tag for the engine, such as fr-CA.
        #[arg(long)]
        language: Option<String>,
        /// Show what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// List a transcript's open review flags, or resolve one.
    Review(review::Args),
    /// Confirm a term, or confirm or reject a recording's tag or relation.
    Confirm(confirm::Args),
    /// Manage tags across every note vpt owns.
    #[command(subcommand)]
    Tag(groups::Tag),
    /// Analyze one recording, every recording, or ids read from stdin.
    Analyze(analyze::Args),
    /// Print the saved path of a recording's stage or an occasion's brief.
    Path(groups::Path),
    /// Write notes from saved state.
    #[command(subcommand)]
    Note(groups::Note),
    /// Check that a note matches its recording.
    VerifyNote(groups::VerifyNote),
    /// List saved occasions.
    Occasions,
    /// List recordings, optionally only those at one stage.
    List {
        /// Only recordings at this stage: transcribe, note or analysis.
        #[arg(long)]
        stage: Option<String>,
    },
    /// Search transcripts for words.
    Search(search::Options),
    /// Report file counts and sizes for each store.
    Storage,
    /// Remove files past their retention.
    #[command(subcommand)]
    Retention(groups::Retention),
    /// Write or preview a meeting brief.
    Brief(brief::Args),
    /// Release a redacted copy of a transcript or analysis.
    Redact(redact::Options),
    /// Manage the home link.
    #[command(subcommand)]
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
