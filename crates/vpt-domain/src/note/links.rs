use super::RenderError;
use std::path::{Path, PathBuf};
mod destination;
mod wiki;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStyle {
    Markdown,
    Wiki,
}

pub fn reference(
    style: LinkStyle,
    label: &str,
    target: &Path,
    origin: &Path,
    wiki_root: Option<&Path>,
) -> Result<String, RenderError> {
    if label.len() > 262_144 || label.chars().count() > 65_536 {
        return Err(RenderError::CapacityExceeded);
    }
    let escaped = super::escape_text(label)?;
    destination::validate(target)?;
    destination::validate(origin)?;
    if style == LinkStyle::Wiki
        && let Some(path) = wiki_root.and_then(|root| wiki::target(target, root))
    {
        return Ok(if path == label {
            format!("[[{path}]]")
        } else {
            format!("[[{path}|{escaped}]]")
        });
    }
    Ok(format!(
        "[{escaped}]({})",
        destination::relative(target, origin)?
    ))
}
