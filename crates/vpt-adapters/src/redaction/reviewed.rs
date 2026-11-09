use super::*;
pub use vpt_domain::redaction::FlagPolicy;
use vpt_domain::{review::ReviewFlag, transcript::Transcript};
pub enum ReviewSource<'a> {
    Transcript {
        transcript: &'a Transcript,
        flags: &'a [ReviewFlag],
    },
    Analysis {
        flags: &'a [ReviewFlag],
    },
}
mod map;
mod plan;
pub fn redact_reviewed(
    source: &str,
    review: ReviewSource<'_>,
    policy: FlagPolicy,
    redactor: &mut Redactor,
    options: &ContentOptions,
) -> Result<Content, Error> {
    let plan = plan::make(source, review, policy)?;
    let mapped = map::Mapped::apply(source, plan.edits)?;
    let mut content = super::render(&mapped.text, &mapped.markers, redactor, options)?;
    for line in &mut content.lines {
        line.source = mapped.original(line.source.clone())?;
    }
    match policy {
        FlagPolicy::Omit => content.omitted = plan.count,
        FlagPolicy::Mark => content.marked = plan.count,
    }
    Ok(content)
}
#[cfg(test)]
mod tests;
