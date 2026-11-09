use super::*;

pub(super) fn write(output: &mut Writer, note: &Note<'_>) -> Result<(), RenderError> {
    output.literal("\n## Open tasks\n\n")?;
    let mut any = false;
    for task in note.document.tasks.iter().filter(|task| !task.completed) {
        any = true;
        output.literal("- \"")?;
        output.text(&task.content)?;
        output.literal(&format!("\" — {}; project: \"", task.source.as_str()))?;
        output.text(&task.project)?;
        output.literal("\"; id: \"")?;
        output.text(&task.id)?;
        output.literal("\"")?;
        if let Some(due) = &task.due {
            output.literal("; due: \"")?;
            output.text(due)?;
            output.literal("\"")?;
        }
        if let Some(priority) = task.priority {
            output.literal(&format!("; priority: {priority}"))?;
        }
        output.literal("\n")?;
    }
    if !any {
        output.literal("None.\n")?;
    }
    Ok(())
}
