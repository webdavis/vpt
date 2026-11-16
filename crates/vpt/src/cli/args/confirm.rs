use super::{Verb, examples, text::PrivateText};
use vpt_domain::tags::Action;

#[derive(clap::Subcommand)]
pub(super) enum Confirm {
    /// Add a confirmed term, used by every later transcription.
    #[command(after_long_help = examples::CONFIRM_TERM)]
    Term {
        /// The term, taken as typed.
        #[arg(allow_hyphen_values = true)]
        term: PrivateText,
    },
    /// Confirm a proposed tag of a recording.
    #[command(after_long_help = examples::CONFIRM_TAG)]
    Tag {
        /// The recording id.
        id: String,
        /// The tag, taken as typed.
        #[arg(allow_hyphen_values = true)]
        tag: PrivateText,
    },
    /// Reject a proposed tag of a recording.
    #[command(after_long_help = examples::CONFIRM_REJECT_TAG)]
    RejectTag {
        /// The recording id.
        id: String,
        /// The tag, taken as typed.
        #[arg(allow_hyphen_values = true)]
        tag: PrivateText,
    },
    /// Confirm a relation of a recording.
    #[command(after_long_help = examples::CONFIRM_RELATION)]
    Relation {
        /// The recording id.
        id: String,
        /// The relation, as <kind>:<target>.
        #[arg(value_name = "KIND:TARGET", allow_hyphen_values = true)]
        relation: PrivateText,
    },
}

impl Confirm {
    pub(super) fn verb(self) -> Verb {
        match self {
            Self::Term { term } => Verb::ConfirmTerm { term },
            Self::Tag { id, tag } => Verb::ConfirmTag {
                id,
                tag,
                action: Action::Confirm,
            },
            Self::RejectTag { id, tag } => Verb::ConfirmTag {
                id,
                tag,
                action: Action::Reject,
            },
            Self::Relation { id, relation } => Verb::ConfirmRelation { id, relation },
        }
    }
}
