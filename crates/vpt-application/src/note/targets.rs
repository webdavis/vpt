use super::*;

pub(super) struct Target<'a> {
    pub stage: NoteStage,
    pub path: PathBuf,
    pub current: Option<StoredNote>,
    pub previous: Option<&'a NoteArtifact>,
    pub first_written: UtcInstant,
    pub write_offset: UtcOffset,
}

pub(super) fn locate<'a>(
    input: &'a NoteInputs,
    settings: &Settings,
    options: &WriteOptions<'_>,
    at: UtcInstant,
    offset: UtcOffset,
    stores: &(impl Stores + NoteFiles),
    format: &impl NoteFormat,
) -> Result<Vec<Target<'a>>, NoteWriteError> {
    let mut targets = Vec::new();
    for (stage, root) in [
        (NoteStage::Transcript, &options.roots.transcripts),
        (NoteStage::Analysis, &options.roots.analysis),
    ] {
        let previous = input.notes.iter().find(|note| note.stage == stage);
        let state = match stage {
            NoteStage::Transcript => input.record.stages.note,
            NoteStage::Analysis => input.record.stages.analysis,
        };
        if (stage == NoteStage::Analysis
            && previous.is_none()
            && options.analysis_content.is_none())
            || (!options.regenerate_expired && state == StageState::Expired)
        {
            continue;
        }
        let path = match previous {
            Some(note) => note.target.clone(),
            None => root.join(
                note::filename::transcript_name(
                    &input.record.id,
                    input.record.title.as_deref(),
                    settings.note.slug_max_chars as usize,
                )
                .map_err(NoteWriteError::Render)?,
            ),
        };
        let (path, current) = super::locate::current(previous, path, root, stores, format)?;
        targets.push(Target {
            stage,
            path,
            current,
            previous,
            first_written: previous.map_or(at, |note| note.first_written),
            write_offset: previous.map_or(offset, |note| note.write_offset),
        });
    }
    Ok(targets)
}
