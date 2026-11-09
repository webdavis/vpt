use super::*;
use vpt_domain::review::ReviewFlag;
pub(super) fn snapshot(
    source: &Source,
    stage: NoteStage,
    flags: &[ReviewFlag],
) -> Result<ReleaseProvenance, Box<ErrorDocument>> {
    let accepted = source
        .input
        .review
        .accepted
        .as_ref()
        .ok_or_else(errors::missing)?;
    let mut lexical = 0;
    let mut total = 0;
    let mut diagnostics = Vec::new();
    for flag in flags.iter().filter(|f| f.resolution == Resolution::Open) {
        match &flag.content {
            FlagContent::Lexical(flag) if flag.class.surfaced() => {
                lexical += 1;
                total += 1;
            }
            FlagContent::Verification(_) => total += 1,
            FlagContent::Diagnostic(diagnostic) if !diagnostics.contains(diagnostic) => {
                diagnostics.push(*diagnostic)
            }
            _ => (),
        }
    }
    let reviewed = accepted.lexical_classification
        && total == 0
        && diagnostics.is_empty()
        && (stage == NoteStage::Analysis || !source.input.review.stale_artifacts.contains(&stage));
    Ok(ReleaseProvenance {
        captured_at: source.input.record.captured_at,
        captured_offset: source.input.record.captured_offset,
        engines: std::iter::once(&accepted.transcript.engine)
            .chain(accepted.checker_engine.as_ref())
            .map(|e| format!("{}:{}", e.name, e.model))
            .collect(),
        open_lexical_flags: lexical,
        open_flags: total,
        diagnostics,
        reviewed,
    })
}
