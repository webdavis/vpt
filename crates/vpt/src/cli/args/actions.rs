use super::{Verb, examples};

#[derive(clap::Subcommand)]
pub(super) enum Actions {
    /// List each follow-up and its state: pending, sent, refused or withdrawing.
    #[command(after_long_help = examples::ACTIONS_LIST)]
    List {
        /// Only this recording's follow-ups.
        id: Option<String>,
    },
    /// Send again every follow-up the task tool refused, for all recordings or for one.
    #[command(after_long_help = examples::ACTIONS_RESEND)]
    Resend {
        /// Only this recording's follow-ups.
        id: Option<String>,
        /// List what would be sent, and send nothing.
        #[arg(long, short = 'n')]
        preview: bool,
    },
}

impl Actions {
    pub(super) fn verb(self) -> Verb {
        match self {
            Self::List { id } => Verb::ActionsList { id },
            Self::Resend { id, preview } => Verb::ActionsResend { id, preview },
        }
    }
}
