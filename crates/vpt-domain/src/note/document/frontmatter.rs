use super::*;

pub(super) fn write(output: &mut Writer, note: &RecordingNote<'_>) -> Result<(), RenderError> {
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
        yaml::list(output, "tags", note.tags.iter().map(String::as_str))?;
        yaml::field(output, "status", "active")?;
        yaml::field(
            output,
            "description",
            &format!("Voice memo recorded {}", note.recording.capture_date()),
        )?;
        yaml::field(
            output,
            "start-date",
            &format!("{:04}-{:02}-{:02}", local.year, local.month, local.day),
        )?;
    }
    output.literal("vpt-schema: 1\n")?;
    yaml::field(output, "vpt-recording", note.recording.as_str())?;
    yaml::field(output, "vpt-stage", note.stage.as_str())?;
    yaml::field(
        output,
        "vpt-captured-at",
        &note.captured_at.rfc3339_with(note.offset),
    )?;
    output.literal(&format!("vpt-duration-secs: {}\n", note.duration_secs))?;
    yaml::field(output, "vpt-language", note.language)?;
    output.literal(&format!(
        "vpt-open-flags: {}\nvpt-diagnostics: {}\n",
        note.open_flags, note.diagnostics
    ))?;
    yaml::list(
        output,
        "vpt-engines",
        note.engines.iter().map(String::as_str),
    )?;
    output.literal(&format!("vpt-readability: {}\n", note.readability))?;
    if matches!(note.profile, Profile::Portable) {
        yaml::list(output, "tags", note.tags.iter().map(String::as_str))?;
    }
    yaml::list(
        output,
        "vpt-suggested-tags",
        note.suggested_tags.iter().map(String::as_str),
    )?;
    output.literal("---\n")
}
