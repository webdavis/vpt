use super::*;

pub(super) fn current(
    previous: Option<&NoteArtifact>,
    target: PathBuf,
    root: &Path,
    stores: &(impl Stores + NoteFiles),
    format: &impl NoteFormat,
) -> Result<(PathBuf, Option<StoredNote>), NoteWriteError> {
    if target.parent() != Some(root) {
        return Err(NoteWriteError::Store(StoreError::Escape(target)));
    }
    stores
        .validate_path(&target)
        .map_err(NoteWriteError::Store)?;
    let current = stores.read_note(&target).map_err(NoteWriteError::Store)?;
    if previous.is_none() && current.is_some() {
        return Err(NoteWriteError::Foreign(target));
    }
    let Some(previous) = previous.filter(|_| current.is_none()) else {
        return Ok((target, current));
    };
    let entries = stores.note_entries(root).map_err(NoteWriteError::Store)?;
    if let Some(path) = entries.redirected.iter().find(|path| markdown(path)) {
        return Err(NoteWriteError::Store(StoreError::Escape(path.clone())));
    }
    let mut remaining = 256 * 1024 * 1024u64;
    let mut claimants = Vec::new();
    let mut found = None;
    for entry in entries.files.iter().filter(|entry| markdown(&entry.path)) {
        remaining = remaining
            .checked_sub(entry.size)
            .ok_or_else(|| NoteWriteError::Format {
                path: root.into(),
                error: NoteFormatError::CapacityExceeded,
            })?;
        let note = stores
            .read_note(&entry.path)
            .map_err(NoteWriteError::Store)?
            .ok_or_else(|| changed(&entry.path))?;
        if note.text.len() as u64 != entry.size {
            return Err(changed(&entry.path));
        }
        let recording = format
            .recording(&note.text)
            .map_err(|error| NoteWriteError::Format {
                path: entry.path.clone(),
                error,
            })?;
        if recording.as_ref() == Some(&previous.recording) {
            claimants.push(entry.path.clone());
            if claimants.len() == 1 {
                found = Some(note);
            }
        }
    }
    if stores.note_entries(root).map_err(NoteWriteError::Store)? != entries {
        return Err(changed(root));
    }
    if claimants.len() > 1 {
        return Err(NoteWriteError::Ambiguous(claimants));
    }
    match (claimants.pop(), found) {
        (Some(path), Some(note)) => {
            if stores
                .read_note(&path)
                .map_err(NoteWriteError::Store)?
                .as_ref()
                != Some(&note)
            {
                return Err(changed(&path));
            }
            Ok((path, Some(note)))
        }
        _ => Ok((target, None)),
    }
}

fn markdown(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}
fn changed(path: &Path) -> NoteWriteError {
    NoteWriteError::Store(StoreError::Io(format!(
        "note store changed during discovery: {}",
        path.display()
    )))
}
