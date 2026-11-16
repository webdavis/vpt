use super::Verb;

#[derive(clap::Args)]
pub(super) struct Run {
    /// Show what would happen without changing anything.
    #[arg(long = "preview", short = 'n')]
    dry_run: bool,
}

#[derive(clap::Args)]
pub(super) struct Transcribe {
    /// The recording id.
    id: String,
    /// Language tag for the engine, such as fr-CA.
    #[arg(long)]
    language: Option<String>,
    /// Show what would happen without changing anything.
    #[arg(long = "preview", short = 'n')]
    dry_run: bool,
}

impl Run {
    pub(super) fn verb(self) -> Verb {
        Verb::Run {
            dry_run: self.dry_run,
        }
    }
}

impl Transcribe {
    pub(super) fn verb(self) -> Verb {
        Verb::Transcribe {
            id: self.id,
            language: self.language,
            dry_run: self.dry_run,
        }
    }
}
