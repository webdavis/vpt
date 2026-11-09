use super::{errors, inspect::Source};
pub(super) use verification::Checked;
use vpt_adapters::{note_document::Document, verification};
use vpt_application::ports::NoteInputs;
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    source: &Source,
    input: &NoteInputs,
    window: u32,
) -> Result<Checked, Box<ErrorDocument>> {
    let accepted = input
        .review
        .accepted
        .as_ref()
        .ok_or_else(|| errors::usage("recording has no accepted transcript"))?;
    let document = source
        .stage
        .map(|_| Document::parse(&source.note.text))
        .transpose()
        .map_err(|_| errors::content())?;
    let content = document
        .as_ref()
        .map(|doc| doc.content())
        .transpose()
        .map_err(|_| errors::content())?
        .unwrap_or(&source.note.text);
    let line_offset = document
        .as_ref()
        .map_or(0, |doc| doc.content_start_line() - 1);
    let mut checked = verification::check(
        content,
        source.stage,
        accepted,
        &input.review.flags_for_recheck(),
        window,
    )
    .map_err(|_| errors::content())?;
    for finding in &mut checked.report {
        finding.line += line_offset;
    }
    Ok(checked)
}
