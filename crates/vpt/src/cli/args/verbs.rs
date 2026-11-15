use super::{
    PathBuf, Verb, analyze, brief, completion, confirm, examples, groups, import, ledger, pipeline,
    redact, review, search, show,
};

#[derive(clap::Subcommand)]
pub(super) enum Command {
    /// Ingest, transcribe, publish and analyze every new recording.
    #[command(after_long_help = examples::RUN)]
    Run(pipeline::Run),
    /// Choose the main engine and create the private directories.
    #[command(after_long_help = examples::SETUP)]
    Setup {
        /// Replace an existing configuration file.
        #[arg(long)]
        overwrite: bool,
        /// Use this main engine instead of asking for it.
        #[arg(long, value_name = "NAME", value_parser = clap::builder::PossibleValuesParser::new(crate::commands::setup::ENGINES))]
        engine: Option<String>,
    },
    /// Read this machine and report what needs attention, contacting nothing.
    #[command(after_long_help = examples::DOCTOR)]
    Doctor {
        /// Run only these checks.
        #[arg(value_name = "CHECK", value_parser = clap::builder::PossibleValuesParser::new(crate::doctor::NAMES))]
        checks: Vec<String>,
    },
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
    Transcribe(pipeline::Transcribe),
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
    /// See whether the ledger is the one this machine last wrote, and accept it.
    #[command(subcommand, after_long_help = examples::DOCS)]
    Ledger(ledger::Action),
    /// Print the vpt and helper versions.
    #[command(after_long_help = examples::VERSION)]
    Version,
    /// Print a shell completion script.
    #[command(after_long_help = examples::COMPLETION)]
    Completion(completion::Completion),
}

impl Command {
    pub(super) fn verb(self) -> Result<Verb, String> {
        Ok(match self {
            Self::Run(run) => run.verb(),
            Self::Setup { overwrite, engine } => Verb::Setup { overwrite, engine },
            Self::Doctor { checks } => Verb::Doctor { checks },
            Self::Ingest { dry_run, once } => Verb::Ingest { dry_run, once },
            Self::Import(options) => return options.verb(),
            Self::Show(args) => return args.verb(),
            Self::Transcribe(transcribe) => transcribe.verb(),
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
            Self::Ledger(action) => Verb::Ledger(action),
            Self::Version => Verb::Version,
            Self::Completion(completion) => completion.verb(),
        })
    }
}
