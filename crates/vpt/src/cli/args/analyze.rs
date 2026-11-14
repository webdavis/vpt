use super::{Verb, values};
use vpt_domain::time::UtcInstant;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    All { since: Option<UtcInstant> },
    Stdin,
}

#[derive(clap::Args)]
pub(super) struct Args {
    /// The recording id.
    #[arg(
        required_unless_present_any = ["all", "stdin"],
        conflicts_with_all = ["all", "stdin"]
    )]
    id: Option<String>,
    /// Show what would happen without changing anything.
    #[arg(long)]
    dry_run: bool,
    /// Send actions the action command refused before.
    #[arg(long, conflicts_with_all = ["all", "stdin"])]
    resend_refused: bool,
    /// Analyze every recording with a transcript.
    #[arg(long, conflicts_with = "stdin")]
    all: bool,
    /// Read one recording id per line from stdin.
    #[arg(long)]
    stdin: bool,
    /// With --all, only recordings captured at or after this RFC 3339 time.
    #[arg(
        long,
        requires = "all",
        conflicts_with_all = ["id", "stdin"],
        value_parser = values::instant
    )]
    since: Option<UtcInstant>,
}

impl Args {
    pub(super) fn verb(self) -> Verb {
        let scope = match self.id {
            Some(id) => {
                return Verb::Analyze {
                    id,
                    dry_run: self.dry_run,
                    resend_refused: self.resend_refused,
                };
            }
            None if self.all => Scope::All { since: self.since },
            None => Scope::Stdin,
        };
        Verb::AnalyzeBatch {
            scope,
            dry_run: self.dry_run,
        }
    }
}
