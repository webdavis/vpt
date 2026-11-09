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
            "startDate",
            &format!("{:04}-{:02}-{:02}", local.year, local.month, local.day),
        )?;
    }
    output.literal("vptSchema: 1\n")?;
    yaml::field(output, "vptRecording", note.recording.as_str())?;
    yaml::field(output, "vptStage", note.stage.as_str())?;
    yaml::field(
        output,
        "vptCapturedAt",
        &note.captured_at.rfc3339_with(note.offset),
    )?;
    output.literal(&format!("vptDurationSecs: {}\n", note.duration_secs))?;
    yaml::field(output, "vptLanguage", note.language)?;
    output.literal(&format!(
        "vptOpenFlags: {}\nvptDiagnostics: {}\n",
        note.open_flags, note.diagnostics
    ))?;
    yaml::list(
        output,
        "vptEngines",
        note.engines.iter().map(String::as_str),
    )?;
    output.literal(&format!("vptReadability: {}\n", note.readability))?;
    if matches!(note.profile, Profile::Portable) {
        yaml::list(output, "tags", note.tags.iter().map(String::as_str))?;
    }
    yaml::list(
        output,
        "vptSuggestedTags",
        note.suggested_tags.iter().map(String::as_str),
    )?;
    output.literal("---\n")
}
