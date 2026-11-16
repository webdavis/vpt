use super::{Verb, text::PrivateText, values};
use vpt_domain::occasion::{OccasionDetails, OccasionId, Participant, time::OccasionTime};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Existing(OccasionId),
    Manual(OccasionDetails),
    Upcoming,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub target: Target,
    pub dry_run: bool,
    pub explain: bool,
}

const CAPACITY: usize = 256;
const MANUAL: [&str; 2] = ["id", "upcoming"];

#[derive(clap::Args)]
pub(super) struct Args {
    /// The saved occasion id.
    #[arg(
        value_name = "OCCASION_ID",
        value_parser = occasion_id,
        required_unless_present_any = ["title", "upcoming"],
        conflicts_with_all = ["title", "upcoming"]
    )]
    id: Option<OccasionId>,
    /// Show the brief without writing it.
    #[arg(long = "preview", short = 'n')]
    dry_run: bool,
    /// Show the brief's sections and where each came from.
    #[arg(long)]
    explain: bool,
    /// Write briefs for the meetings that start soon.
    #[arg(long, conflicts_with_all = ["title", "dry_run", "explain"])]
    upcoming: bool,
    /// Create a manual occasion with this title.
    #[arg(long, requires = "at", allow_hyphen_values = true)]
    title: Option<PrivateText>,
    /// The manual occasion's start, as an RFC 3339 time.
    #[arg(long, requires = "title", conflicts_with_all = MANUAL, value_parser = values::time)]
    at: Option<OccasionTime>,
    /// The manual occasion's length in minutes.
    #[arg(
        long = "duration-minutes",
        value_name = "MINUTES",
        requires = "title",
        conflicts_with_all = MANUAL,
        value_parser = seconds
    )]
    duration_secs: Option<u64>,
    /// A participant of the manual occasion; repeat for more.
    #[arg(
        long,
        value_name = "NAME",
        requires = "title",
        conflicts_with_all = MANUAL,
        allow_hyphen_values = true
    )]
    participant: Vec<PrivateText>,
    /// A tag of the manual occasion; repeat for more.
    #[arg(long, requires = "title", conflicts_with_all = MANUAL, allow_hyphen_values = true)]
    tag: Vec<PrivateText>,
}

fn occasion_id(text: &str) -> Result<OccasionId, String> {
    OccasionId::parse(text).map_err(|_| "invalid occasion id".into())
}

fn seconds(text: &str) -> Result<u64, String> {
    text.parse::<u64>()
        .ok()
        .and_then(|minutes| minutes.checked_mul(60))
        .ok_or_else(|| "expected a whole number of minutes".into())
}

impl Args {
    pub(super) fn verb(self) -> Result<Verb, String> {
        for (flag, items) in [("--tag", &self.tag), ("--participant", &self.participant)] {
            if items.len() > CAPACITY {
                return Err(format!("{flag} exceeds capacity"));
            }
        }
        let target = match (self.id, self.title, self.at) {
            (Some(id), ..) => Target::Existing(id),
            (None, Some(title), Some(at)) => {
                let details = OccasionDetails {
                    at,
                    title: title.as_str().into(),
                    duration_secs: self.duration_secs,
                    participants: self
                        .participant
                        .iter()
                        .map(|name| Participant {
                            name: name.as_str().into(),
                            email: None,
                        })
                        .collect(),
                    tags: self.tag.iter().map(|tag| tag.as_str().into()).collect(),
                };
                if !details.is_valid() {
                    return Err("invalid manual occasion".into());
                }
                Target::Manual(details)
            }
            _ => Target::Upcoming,
        };
        Ok(Verb::Brief(Options {
            target,
            dry_run: self.dry_run,
            explain: self.explain,
        }))
    }
}
