use super::{Verb, examples, values};
use vpt_domain::time::UtcInstant;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    All { since: Option<UtcInstant> },
    Stdin,
}

const STDIN: &str = "-";

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub(super) struct Args {
    /// The recording id, or - to read one id per line from stdin.
    #[arg(required = true)]
    id: Option<String>,
    /// Show what would happen without changing anything.
    #[arg(long = "preview", short = 'n')]
    dry_run: bool,
    #[command(subcommand)]
    all: Option<All>,
}

#[derive(clap::Subcommand)]
enum All {
    /// Analyze every recording with a transcript.
    #[command(after_long_help = examples::ANALYZE_ALL)]
    All {
        /// Only recordings captured at or after this RFC 3339 time.
        #[arg(long, value_parser = values::instant)]
        since: Option<UtcInstant>,
        /// Show what would happen without changing anything.
        #[arg(long = "preview", short = 'n')]
        dry_run: bool,
    },
}

impl Args {
    pub(super) fn verb(self) -> Verb {
        match (self.all, self.id) {
            (Some(All::All { since, dry_run }), _) => Verb::AnalyzeBatch {
                scope: Scope::All { since },
                dry_run,
            },
            (None, Some(id)) if id == STDIN => Verb::AnalyzeBatch {
                scope: Scope::Stdin,
                dry_run: self.dry_run,
            },
            (None, id) => Verb::Analyze {
                id: id.unwrap_or_default(),
                dry_run: self.dry_run,
            },
        }
    }
}
