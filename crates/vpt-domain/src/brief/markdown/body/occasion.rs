use super::*;

pub(super) fn write(output: &mut Writer, note: &Note<'_>) -> Result<(), RenderError> {
    let document = &note.document;
    let occasion = document.occasion;
    output.literal("## Occasion\n\n- Title: \"")?;
    output.text(&occasion.title)?;
    output.literal(&format!(
        "\" ({})\n- At: {}\n",
        document.source.as_str(),
        occasion.at.as_str()
    ))?;
    if let Some(duration) = occasion.duration_secs {
        output.literal(&format!("- Duration: {duration} seconds\n"))?;
    }
    for (person, matched) in occasion
        .participants
        .iter()
        .zip(&document.pack.selection.participants)
    {
        output.literal("- Participant: \"")?;
        output.text(&person.name)?;
        if let Some(email) = &person.email {
            output.literal("; ")?;
            output.text(email)?;
        }
        output.literal(&format!("\" ({})", matched.term.as_str()))?;
        if let Some(path) = &matched.matched_note {
            output.literal("; ")?;
            output.literal(&reference(note, path)?)?;
        }
        output.literal("\n")?;
    }
    for tag in &occasion.tags {
        output.literal("- Tag: \"")?;
        output.text(tag)?;
        output.literal("\"\n")?;
    }
    for (name, status) in [
        ("calendar", &document.context.calendar),
        ("tasks", &document.context.tasks),
    ] {
        output.literal(&format!("- {name}: {}", status.status()))?;
        if let Some(reason) = status.reason() {
            output.literal(&format!(" ({reason})"))?;
        }
        if let Some(detail) = status.detail() {
            output.literal("; \"")?;
            output.text(detail)?;
            output.literal("\"")?;
        }
        output.literal("\n")?;
    }
    Ok(())
}
