mod occasion;
mod selection;
mod tasks;
use super::*;

pub(super) fn write(output: &mut Writer, note: &Note<'_>) -> Result<(), RenderError> {
    occasion::write(output, note)?;
    output.literal("\n## Unresolved\n\n")?;
    let pack = note.document.pack;
    if pack.unresolved.flags == 0 && pack.unresolved.reviewed {
        output.literal("None.\n")?;
    } else {
        output.literal(&format!(
            "- Open flags: {}\n- Notes with flags: {}\n- Reviewed: {}\n",
            pack.unresolved.flags, pack.unresolved.notes_with_flags, pack.unresolved.reviewed
        ))?;
    }
    output.literal("\n## From your notes\n\n")?;
    if pack.items.is_empty() {
        output.literal("None.\n")?;
    }
    for quote in &pack.items {
        output.literal(&format!("- [{}] ", timecode(quote.source.at_secs)))?;
        if quote.certainty == Certainty::Unverified {
            output.literal("[unverified] ")?;
        }
        output.literal("\"")?;
        output.text(&quote.text)?;
        output.literal("\" — ")?;
        output.literal(quote.source.recording.as_str())?;
        output.literal("; ")?;
        output.literal(&reference(note, &quote.source.note)?)?;
        output.literal(&format!("; {}\n", quote.selector.as_str()))?;
    }
    tasks::write(output, note)?;
    selection::write(output, note)
}

fn timecode(seconds: f64) -> String {
    let whole = seconds as u64;
    let number = seconds.to_string();
    let fraction = number.find('.').map_or("", |at| &number[at..]);
    format!("{:02}:{:02}{fraction}", whole / 60, whole % 60)
}
