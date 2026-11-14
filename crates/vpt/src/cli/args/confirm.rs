use super::{Verb, text::PrivateText};
use vpt_domain::tags::Action;

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("confirmation")
        .required(true)
        .args(["term", "tag", "reject_tag", "relation"])
))]
pub(super) struct Args {
    #[arg(required_unless_present = "term", conflicts_with = "term")]
    id: Option<String>,
    /// Add a confirmed term; takes no recording id.
    #[arg(long, allow_hyphen_values = true)]
    term: Option<PrivateText>,
    /// Confirm a proposed tag of the recording.
    #[arg(long, allow_hyphen_values = true)]
    tag: Option<PrivateText>,
    /// Reject a proposed tag of the recording.
    #[arg(long, allow_hyphen_values = true)]
    reject_tag: Option<PrivateText>,
    /// Confirm a relation of the recording, as <kind>:<target>.
    #[arg(long, allow_hyphen_values = true)]
    relation: Option<PrivateText>,
}

impl Args {
    pub(super) fn verb(self) -> Verb {
        if let Some(term) = self.term {
            return Verb::ConfirmTerm { term };
        }
        let id = self.id.unwrap_or_default();
        if let Some(relation) = self.relation {
            return Verb::ConfirmRelation { id, relation };
        }
        let (tag, action) = match (self.tag, self.reject_tag) {
            (Some(tag), _) => (tag, Action::Confirm),
            (None, tag) => (tag.unwrap_or_else(|| String::new().into()), Action::Reject),
        };
        Verb::ConfirmTag { id, tag, action }
    }
}
