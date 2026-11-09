use super::*;

pub(super) fn append(
    delta: &mut WorkDelta,
    input: &NoteInputs,
    target: &targets::Target<'_>,
    rendered: render::Rendered,
    at: UtcInstant,
    stores: &impl Stores,
    format: &impl NoteFormat,
) -> Result<(), NoteWriteError> {
    let source = target
        .current
        .as_ref()
        .map(|note| note.text.as_str())
        .or_else(|| target.previous.map(|note| note.text.as_str()));
    let text = match source {
        Some(source) => format
            .rewrite_owned(
                source,
                &rendered.text,
                &input.record.id,
                target.stage,
                target
                    .previous
                    .map(|note| note.managed_tags.as_slice())
                    .unwrap_or_default(),
            )
            .map_err(|error| NoteWriteError::Format {
                path: target.path.clone(),
                error,
            })?,
        None => rendered.text,
    };
    let artifact = NoteArtifact {
        recording: input.record.id.clone(),
        stage: target.stage,
        target: target.path.clone(),
        digest: stores.digest_bytes(text.as_bytes()),
        content_digest: stores.digest_bytes(rendered.content.as_bytes()),
        text,
        managed_tags: rendered.managed_tags,
        first_written: target.first_written,
        write_offset: target.write_offset,
        updated_at: at,
    };
    delta.commit.publications.push(DirtyPublication {
        target: target.path.clone(),
        expected_previous: target.current.as_ref().map(|note| note.digest),
        intended: artifact.digest,
        recorded_at: at,
    });
    delta.notes.push(NoteChange {
        relocated_from: target
            .previous
            .filter(|note| note.target != target.path)
            .map(|note| note.target.clone()),
        artifact,
        expected_saved: target.previous.map(|note| note.digest),
    });
    Ok(())
}
