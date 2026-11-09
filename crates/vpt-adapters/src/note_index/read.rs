use super::*;

pub(super) fn walk(
    root: &RootDir,
    index: &mut Index,
    limits: &mut Limits,
    depth: usize,
) -> Result<(), StoreError> {
    if depth > limits.depth {
        return Err(capacity());
    }
    let names = root.names_bounded(limits.entries).map_err(contained)?;
    limits.entries = limits
        .entries
        .checked_sub(names.len())
        .ok_or_else(capacity)?;
    let stores =
        FilesystemStores::open_read_only(&[root.path().to_path_buf()]).map_err(contained)?;
    for name in &names {
        let stat = root.stat(Path::new(name)).map_err(contained)?;
        match stat.kind {
            Kind::Directory => walk(
                &root.open_subdirectory(name).map_err(contained)?,
                index,
                limits,
                depth + 1,
            )?,
            Kind::File
                if Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md")) =>
            {
                let path = root.path().join(name);
                let stored = stores
                    .read_note_bounded(&path, limits.read_bytes)?
                    .ok_or_else(changed)?;
                limits.read_bytes = limits
                    .read_bytes
                    .checked_sub(stored.text.len())
                    .ok_or_else(capacity)?;
                let metadata = match metadata::read(&stored.text) {
                    Ok(metadata) => metadata,
                    Err(_) => {
                        limits.metadata_bytes = limits
                            .metadata_bytes
                            .checked_sub(path.as_os_str().len() + std::mem::size_of::<Diagnostic>())
                            .ok_or_else(capacity)?;
                        index.diagnostics.push(Diagnostic {
                            path: path.clone(),
                            kind: DiagnosticKind::Metadata,
                        });
                        metadata::Metadata::default()
                    }
                };
                let name = Path::new(name)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or_else(changed)?
                    .to_owned();
                let bytes = metadata
                    .aliases
                    .iter()
                    .chain(&metadata.tags)
                    .try_fold(
                        path.as_os_str().len() + name.len() + std::mem::size_of::<IndexedNote>(),
                        |bytes, text| bytes.checked_add(text.len() + std::mem::size_of::<String>()),
                    )
                    .ok_or_else(capacity)?;
                limits.metadata_bytes = limits
                    .metadata_bytes
                    .checked_sub(bytes)
                    .ok_or_else(capacity)?;
                index.notes.push(IndexedNote {
                    path,
                    name,
                    metadata,
                });
            }
            _ => (),
        }
    }
    if root.names_bounded(names.len()).map_err(contained)? != names {
        return Err(changed());
    }
    root.revalidate().map_err(contained)
}

fn changed() -> StoreError {
    StoreError::Io("note index changed during read".into())
}
