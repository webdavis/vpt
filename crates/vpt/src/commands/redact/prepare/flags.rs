use super::*;
use vpt_domain::review::ReviewFlag;

pub(super) fn current(
    source: &Source,
    stage: NoteStage,
    content: &str,
    settings: &Settings,
) -> Result<Vec<ReviewFlag>, Box<ErrorDocument>> {
    if stage == NoteStage::Transcript {
        return Ok(source.input.review.flags.clone());
    }
    let accepted = source
        .input
        .review
        .accepted
        .as_ref()
        .ok_or_else(errors::missing)?;
    let previous = source.input.review.flags_for_recheck();
    let checked = vpt_adapters::verification::check(
        content,
        Some(stage),
        accepted,
        &previous,
        settings.analysis.grounding_window_secs,
    )
    .map_err(|_| errors::source())?;
    let findings =
        vpt_adapters::verification::review_flags(accepted, stage, &checked.findings, &previous)
            .map_err(|_| errors::source())?;
    let mut flags: Vec<_> = source
        .input
        .review
        .flags
        .iter()
        .filter(
            |flag| !matches!(&flag.content, FlagContent::Verification(f) if f.artifact == stage),
        )
        .cloned()
        .collect();
    flags.extend(findings);
    Ok(flags)
}
