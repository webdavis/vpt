use super::*;
use vpt_domain::{
    note::NoteStage,
    review::{FlagContent, Resolution},
};
mod analysis;
mod transcript;
pub(super) struct Plan {
    pub edits: Vec<map::Edit>,
    pub count: usize,
}
pub(super) fn make(
    source: &str,
    review: ReviewSource<'_>,
    policy: FlagPolicy,
) -> Result<Plan, Error> {
    match review {
        ReviewSource::Transcript { transcript, flags } => {
            transcript::plan(source, transcript, flags, policy)
        }
        ReviewSource::Analysis { flags } => analysis::plan(source, flags, policy),
    }
}
fn edit(range: Range<usize>, accepted: String, policy: FlagPolicy) -> map::Edit {
    if policy == FlagPolicy::Omit {
        return map::Edit {
            range,
            text: String::new(),
            marker: None,
        };
    }
    let mut text = accepted;
    if !text.is_empty() {
        text.push(' ');
    }
    let start = text.len();
    text.push_str("`[unverified]`");
    let marker = Some(start..text.len());
    map::Edit {
        range,
        text,
        marker,
    }
}
