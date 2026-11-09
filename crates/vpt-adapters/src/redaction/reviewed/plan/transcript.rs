use super::*;
use std::collections::BTreeSet;
pub(super) fn plan(
    source: &str,
    transcript: &Transcript,
    flags: &[ReviewFlag],
    policy: FlagPolicy,
) -> Result<Plan, Error> {
    if flags.len() > MAX_ITEMS + 3 {
        return Err(Error::Capacity);
    }
    if !flags.iter().any(|flag| {
        matches!(flag.content, FlagContent::Lexical(_)) && flag.resolution == Resolution::Open
    }) {
        return Ok(Plan {
            edits: vec![],
            count: 0,
        });
    }
    let parsed = crate::markdown_claims::extract_owned(source, NoteStage::Transcript, flags)
        .map_err(|_| Error::InvalidText)?;
    let cleared = map::Mapped::apply(
        source,
        parsed
            .annotation_ranges()
            .iter()
            .map(|range| map::Edit {
                range: range.clone(),
                text: String::new(),
                marker: None,
            })
            .collect(),
    )?;
    let mut matched = None;
    for readability in [false, true] {
        let candidate = vpt_domain::note::transcript_with_spans(transcript, flags, readability)
            .map_err(|_| Error::InvalidText)?;
        if candidate.text == cleared.text {
            matched = Some(candidate);
            break;
        }
    }
    let matched = matched.ok_or(Error::InvalidText)?;
    let mut keys = BTreeSet::new();
    let mut edits = Vec::new();
    for span in matched.open_spans {
        keys.insert((span.flag.0, span.occurrence));
        let accepted =
            vpt_domain::note::escape_text(&span.accepted).map_err(|_| Error::Capacity)?;
        edits.push(edit(cleared.original(span.bytes)?, accepted, policy));
    }
    Ok(Plan {
        edits,
        count: keys.len(),
    })
}
