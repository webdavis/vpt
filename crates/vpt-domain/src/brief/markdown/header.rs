use super::*;
use crate::note::document::yaml;

pub(super) fn write(output: &mut Writer, note: &Note<'_>) -> Result<(), RenderError> {
    let document = &note.document;
    output.literal("---\n")?;
    if let Profile::Obsidian {
        title,
        reference,
        hub,
        first_written,
        write_offset,
    } = &note.profile
    {
        let local = first_written.civil(*write_offset);
        if !(0..=9999).contains(&local.year) || write_offset.secs.unsigned_abs() >= 86_400 {
            return Err(RenderError::InvalidMetadata);
        }
        yaml::list(output, "aliases", title.iter().copied())?;
        yaml::field(output, "reference", reference)?;
        yaml::field(output, "hub", hub)?;
        yaml::list(
            output,
            "tags",
            document.occasion.tags.iter().map(String::as_str),
        )?;
        yaml::field(output, "status", "active")?;
        yaml::field(
            output,
            "description",
            &format!("Brief for {}", document.occasion.at.as_str()),
        )?;
        yaml::field(
            output,
            "startDate",
            &format!("{:04}-{:02}-{:02}", local.year, local.month, local.day),
        )?;
    }
    output.literal("vptSchema: 1\n")?;
    yaml::field(output, "vptKind", "brief")?;
    yaml::field(output, "vptOccasion", document.id.as_str())?;
    yaml::field(output, "vptOccasionAt", document.occasion.at.as_str())?;
    yaml::field(output, "vptSource", document.source.as_str())?;
    output.literal(if document.pack.sources.is_empty() {
        "vptNotes: []\n"
    } else {
        "vptNotes:\n"
    })?;
    for source in &document.pack.sources {
        yaml::field(output, "  - recording", source.recording.as_str())?;
        yaml::field(
            output,
            "    note",
            document::source_path(&source.note).ok_or(RenderError::InvalidName)?,
        )?;
        yaml::field(output, "    digest", &source.digest.hex())?;
    }
    let unresolved = &document.pack.unresolved;
    output.literal(&format!(
        "vptUnresolved:\n  flags: {}\n  notes_with_flags: {}\n  reviewed: {}\n",
        unresolved.flags, unresolved.notes_with_flags, unresolved.reviewed
    ))?;
    output.literal("vptContext:\n")?;
    for (name, status) in [
        ("calendar", &document.context.calendar),
        ("tasks", &document.context.tasks),
    ] {
        output.literal(&format!("  {name}:\n"))?;
        yaml::field(output, "    status", status.status())?;
        if let Some(reason) = status.reason() {
            yaml::field(output, "    reason", reason)?;
        } else {
            output.literal("    reason: null\n")?;
        }
        if let Some(detail) = status.detail() {
            yaml::field(output, "    detail", detail)?;
        } else {
            output.literal("    detail: null\n")?;
        }
    }
    yaml::field(output, "vptGeneratedAt", &document.generated_at.rfc3339())?;
    if matches!(note.profile, Profile::Portable) {
        yaml::list(
            output,
            "tags",
            document.occasion.tags.iter().map(String::as_str),
        )?;
    }
    output.literal("---\n")
}
