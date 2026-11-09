use super::*;
pub(super) mod wiki;

pub(super) fn references(
    input: &NoteInputs,
    settings: &Settings,
    target: &Path,
    stage: NoteStage,
    wiki_root: Option<&Path>,
) -> Result<(String, String), NoteWriteError> {
    let origin = target
        .parent()
        .ok_or(NoteWriteError::Render(RenderError::InvalidName))?;
    let audio = reference(
        settings.note.link_style,
        &format!("{}.m4a", input.record.id),
        &input.record.audio_path,
        origin,
        wiki_root,
    )?;
    if settings.note.profile == NoteProfile::Portable {
        return Ok((audio, String::new()));
    }
    let hub = match stage {
        NoteStage::Transcript => &settings.note.obsidian.hub_transcripts,
        NoteStage::Analysis => &settings.note.obsidian.hub_analysis,
    };
    let vault = settings
        .note
        .obsidian
        .vault_root
        .as_deref()
        .ok_or(NoteWriteError::Render(RenderError::InvalidMetadata))?;
    Ok((
        audio,
        reference(
            settings.note.link_style,
            hub,
            &vault.join(format!("{hub}.md")),
            origin,
            wiki_root,
        )?,
    ))
}

pub(super) fn reference(
    style: LinkStyle,
    label: &str,
    target: &Path,
    origin: &Path,
    wiki_root: Option<&Path>,
) -> Result<String, NoteWriteError> {
    vpt_domain::note::links::reference(style, label, target, origin, wiki_root)
        .map_err(NoteWriteError::Render)
}

pub(super) fn counterpart(
    settings: &Settings,
    target: &Path,
    stage: NoteStage,
    other: Option<&Path>,
    wiki_root: Option<&Path>,
) -> Result<String, NoteWriteError> {
    let Some(other) = other else {
        return Ok(String::new());
    };
    let label = other
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or(NoteWriteError::Render(RenderError::InvalidName))?;
    let origin = target
        .parent()
        .ok_or(NoteWriteError::Render(RenderError::InvalidName))?;
    let link = reference(settings.note.link_style, label, other, origin, wiki_root)?;
    let kind = match stage {
        NoteStage::Transcript => "Analysis",
        NoteStage::Analysis => "Transcript",
    };
    Ok(format!("- {kind}: {link}\n"))
}

#[cfg(test)]
mod tests;
