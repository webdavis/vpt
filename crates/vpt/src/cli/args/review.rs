use super::{Verb, examples, text::PrivateText};
use vpt_domain::review::ReviewAction;

#[derive(clap::Subcommand)]
pub(super) enum Review {
    /// List a recording's open review flags.
    #[command(after_long_help = examples::REVIEW_LIST)]
    List {
        /// The recording id.
        id: String,
    },
    /// Accept a flagged text as it is.
    #[command(after_long_help = examples::REVIEW_CONFIRM)]
    Confirm {
        /// The recording id.
        id: String,
        /// The flag to resolve.
        flag: String,
    },
    /// Replace a flagged text with this text.
    #[command(after_long_help = examples::REVIEW_CORRECT)]
    Correct {
        /// The recording id.
        id: String,
        /// The flag to resolve.
        flag: String,
        /// The text that replaces the flagged text.
        #[arg(allow_hyphen_values = true)]
        text: PrivateText,
    },
    /// Dismiss a flag without changing the text.
    #[command(after_long_help = examples::REVIEW_DISMISS)]
    Dismiss {
        /// The recording id.
        id: String,
        /// The flag to resolve.
        flag: String,
    },
}

impl Review {
    pub(super) fn verb(self) -> Verb {
        let (id, flag, action) = match self {
            Self::List { id } => return Verb::Review { id },
            Self::Confirm { id, flag } => (id, flag, ReviewAction::Confirm),
            Self::Correct { id, flag, text } => {
                (id, flag, ReviewAction::Correct(text.as_str().into()))
            }
            Self::Dismiss { id, flag } => (id, flag, ReviewAction::Dismiss),
        };
        Verb::ReviewResolve { id, flag, action }
    }
}
