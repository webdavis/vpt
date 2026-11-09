use super::*;
use std::borrow::Cow;

pub(super) fn prepare<'a>(
    input: &'a NoteInputs,
    settings: &Settings,
    options: &WriteOptions<'_>,
    targets: &[targets::Target<'_>],
    format: &impl NoteFormat,
) -> Result<(Cow<'a, ReviewSnapshot>, Vec<String>), NoteWriteError> {
    let mut review = Cow::Borrowed(&input.review);
    let mut contents = Vec::with_capacity(targets.len());
    for target in targets {
        let content = source(
            &input.review,
            settings,
            target,
            options.analysis_content,
            format,
        )?;
        if let Some(expected) = input
            .review
            .verified_claims
            .iter()
            .find(|claims| claims.artifact == target.stage)
        {
            let actual = format
                .claims_digest(&content, target.stage, &input.review.flags)
                .map_err(|error| NoteWriteError::Format {
                    path: target.path.clone(),
                    error,
                })?;
            if actual != expected.digest {
                review.to_mut().invalidate_artifact(target.stage);
            }
        }
        contents.push(content);
    }
    Ok((review, contents))
}

fn source(
    review: &ReviewSnapshot,
    settings: &Settings,
    target: &targets::Target<'_>,
    analysis_content: Option<&str>,
    format: &impl NoteFormat,
) -> Result<String, NoteWriteError> {
    let accepted = review
        .accepted
        .as_ref()
        .ok_or(NoteWriteError::NoTranscript)?;
    match (target.stage, analysis_content) {
        (NoteStage::Transcript, _) => note::transcript_content(
            &accepted.transcript,
            &review.flags,
            settings.readability_enabled,
        )
        .map_err(NoteWriteError::Render),
        (NoteStage::Analysis, Some(content)) => Ok(content.to_owned()),
        (NoteStage::Analysis, None) => format
            .content(
                &target
                    .previous
                    .ok_or(NoteWriteError::AnalysisUnavailable)?
                    .text,
            )
            .map_err(|error| NoteWriteError::Format {
                path: target.path.clone(),
                error,
            }),
    }
}
