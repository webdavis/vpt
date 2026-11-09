use super::*;

pub(super) fn write(output: &mut Writer, note: &Note<'_>) -> Result<(), RenderError> {
    let pack = note.document.pack;
    output.literal("\n## Not selected\n\n")?;
    let mut any = false;
    for matched in &pack.selection.participants {
        if matched.term == TermState::Confirmed {
            continue;
        }
        any = true;
        let person = &note.document.occasion.participants[matched.participant];
        output.literal("- Participant: \"")?;
        output.text(if person.name.is_empty() {
            person.email.as_deref().unwrap_or("")
        } else {
            &person.name
        })?;
        output.literal(&format!("\"; {}", matched.term.as_str()))?;
        if let Some(command) = &matched.confirmation {
            output.literal("; ")?;
            output.text(command)?;
        }
        output.literal("\n")?;
    }
    for decision in &pack.selection.decisions {
        if !decision.selected {
            any = true;
            output.literal(&format!(
                "- {}: {}\n",
                decision.recording,
                decision.rejected.map_or("note-limit", Rejection::as_str)
            ))?;
        }
    }
    if any || pack.omitted_spans != 0 {
        output.literal(&format!(
            "- Rejected notes: {}\n- Omitted notes: {}\n- Omitted spans: {}\n",
            pack.selection.rejected_notes, pack.selection.omitted_notes, pack.omitted_spans
        ))?;
    } else {
        output.literal("None.\n")?;
    }
    Ok(())
}
