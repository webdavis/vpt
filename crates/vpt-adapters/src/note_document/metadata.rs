use super::{NoteError, header, tree};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

impl std::fmt::Debug for Metadata {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Metadata")
            .field("aliases", &self.aliases.len())
            .field("tags", &self.tags.len())
            .finish()
    }
}

pub fn read(source: &str) -> Result<Metadata, NoteError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    if source.lines().next() != Some("---") {
        return Ok(Metadata::default());
    }
    let (header, _) = header::locate(source)?;
    let header = &source[header];
    if header.trim().is_empty() {
        return Ok(Metadata::default());
    }
    let tree = tree::Tree::parse(header)?;
    let fields = header::fields(&tree)?;
    let mut metadata = Metadata::default();
    for field in fields {
        let target = match field.key.as_str() {
            "aliases" => &mut metadata.aliases,
            "tags" => &mut metadata.tags,
            _ => continue,
        };
        if tree.is_null(field.value_node)? {
            continue;
        }
        *target = match tree.string(field.value_node) {
            Ok("") => Vec::new(),
            Ok(text) => vec![text.to_owned()],
            Err(_) => tree.strings(field.value_node)?,
        };
    }
    Ok(metadata)
}

#[cfg(test)]
mod tests;
