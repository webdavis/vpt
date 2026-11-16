use super::{PathBuf, Verb, examples, text::PrivateText};

#[derive(clap::Subcommand)]
pub(super) enum Tag {
    /// Rename one tag everywhere vpt owns it.
    #[command(after_long_help = examples::TAG_RENAME)]
    Rename {
        /// The tag to rename.
        old: PrivateText,
        /// Its new name.
        new: PrivateText,
        /// Show what would change without changing anything.
        #[arg(long = "preview", short = 'n')]
        dry_run: bool,
    },
}

#[derive(clap::Subcommand)]
pub(super) enum Note {
    /// Write the transcript and analysis notes of one recording.
    #[command(after_long_help = examples::NOTE_WRITE)]
    Write {
        /// The recording id.
        id: String,
    },
}

#[derive(clap::Subcommand)]
pub(super) enum Retention {
    /// Apply the configured retention.
    #[command(after_long_help = examples::RETENTION_RUN)]
    Run {
        /// Show what would be removed without removing anything.
        #[arg(long = "preview", short = 'n')]
        dry_run: bool,
    },
}

#[derive(clap::Subcommand)]
pub(super) enum Symlink {
    /// Create the home link the configuration names.
    #[command(after_long_help = examples::SYMLINK_DEPLOY)]
    Deploy,
    /// Check that the home link points where the configuration says.
    #[command(after_long_help = examples::SYMLINK_VERIFY)]
    Verify,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub(super) struct Path {
    /// The recording id.
    #[arg(required = true)]
    id: Option<String>,
    /// The stage: transcript or analysis.
    #[arg(long, required = true)]
    stage: Option<String>,
    #[command(subcommand)]
    occasion: Option<Occasion>,
}

#[derive(clap::Subcommand)]
enum Occasion {
    /// Print the saved path of an occasion's brief.
    #[command(after_long_help = examples::PATH_OCCASION)]
    Occasion {
        /// The occasion id.
        id: String,
        /// The stage: brief.
        #[arg(long)]
        stage: String,
    },
}

#[derive(clap::Args)]
pub(super) struct VerifyNote {
    /// The note file.
    path: PathBuf,
    /// The recording the note belongs to.
    #[arg(long)]
    recording: String,
}

impl Tag {
    pub(super) fn verb(self) -> Verb {
        let Self::Rename { old, new, dry_run } = self;
        Verb::TagRename { old, new, dry_run }
    }
}

impl Note {
    pub(super) fn verb(self) -> Verb {
        let Self::Write { id } = self;
        Verb::NoteWrite { id }
    }
}

impl Retention {
    pub(super) fn verb(self) -> Verb {
        let Self::Run { dry_run } = self;
        Verb::RetentionRun { dry_run }
    }
}

impl Symlink {
    pub(super) fn verb(self) -> Verb {
        match self {
            Self::Deploy => Verb::SymlinkDeploy,
            Self::Verify => Verb::SymlinkVerify,
        }
    }
}

impl Path {
    pub(super) fn verb(self) -> Verb {
        match self.occasion {
            Some(Occasion::Occasion { id, stage }) => Verb::OccasionPath { id, stage },
            None => Verb::Path {
                id: self.id.unwrap_or_default(),
                stage: self.stage.unwrap_or_default(),
            },
        }
    }
}

impl VerifyNote {
    pub(super) fn verb(self) -> Verb {
        Verb::VerifyNote {
            path: self.path,
            recording: self.recording,
        }
    }
}
