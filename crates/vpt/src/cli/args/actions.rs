use super::{Verb, examples};

#[derive(clap::Subcommand)]
pub(super) enum Actions {
    /// List each follow-up and its state: pending, sent, refused or withdrawing.
    #[command(after_long_help = examples::ACTIONS_LIST)]
    List {
        /// Only this recording's follow-ups.
        id: Option<String>,
    },
}

impl Actions {
    pub(super) fn verb(self) -> Verb {
        match self {
            Self::List { id } => Verb::ActionsList { id },
        }
    }
}
