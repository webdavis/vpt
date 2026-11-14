use super::{Verb, text::PrivateText};
use vpt_domain::review::ReviewAction;

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("resolution")
        .args(["confirm", "correct", "dismiss"])
        .requires("resolve")
))]
pub(super) struct Args {
    id: String,
    /// The flag to resolve, with one of --confirm, --correct or --dismiss.
    #[arg(long, value_name = "FLAG", requires = "resolution")]
    resolve: Option<String>,
    /// Accept the flagged text as it is.
    #[arg(long)]
    confirm: bool,
    /// Replace the flagged text with this text.
    #[arg(long, value_name = "TEXT", allow_hyphen_values = true)]
    correct: Option<PrivateText>,
    /// Dismiss the flag.
    #[arg(long)]
    dismiss: bool,
}

impl Args {
    pub(super) fn verb(self) -> Verb {
        let Some(flag) = self.resolve else {
            return Verb::Review { id: self.id };
        };
        let action = match self.correct {
            Some(text) => ReviewAction::Correct(text.as_str().into()),
            None if self.confirm => ReviewAction::Confirm,
            None => ReviewAction::Dismiss,
        };
        Verb::ReviewResolve {
            id: self.id,
            flag,
            action,
        }
    }
}
