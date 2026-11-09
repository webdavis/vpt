use super::*;
use vpt_application::{LinkStyle, NoteProfile, NoteSettings, ObsidianSettings};

pub(super) fn from_table(table: &Table, home: &Path) -> Result<NoteSettings, ConfigError> {
    let profile = match text(table, "note.profile")? {
        "portable" => NoteProfile::Portable,
        "obsidian" => NoteProfile::Obsidian,
        _ => return Err(invalid("note.profile", "one of portable, obsidian")),
    };
    let link_style = match text(table, "note.link_style")? {
        "markdown" => LinkStyle::Markdown,
        "wiki" => LinkStyle::Wiki,
        _ => return Err(invalid("note.link_style", "one of markdown, wiki")),
    };
    let raw = text(table, "note.obsidian.vault_root")?;
    let vault_root = (!raw.is_empty()).then(|| expand(raw, home, None));
    if profile == NoteProfile::Obsidian
        && vault_root.as_ref().is_none_or(|path| !path.is_absolute())
    {
        return Err(invalid(
            "note.obsidian.vault_root",
            "the obsidian profile requires an absolute vault directory",
        ));
    }
    Ok(NoteSettings {
        profile,
        link_style,
        slug_max_chars: small(table, "note.slug_max_chars")?,
        obsidian: ObsidianSettings {
            vault_root,
            hub_transcripts: text(table, "note.obsidian.hub_transcripts")?.into(),
            hub_analysis: text(table, "note.obsidian.hub_analysis")?.into(),
            hub_briefs: text(table, "note.obsidian.hub_briefs")?.into(),
        },
    })
}

fn invalid(key: &str, rule: &str) -> ConfigError {
    ConfigError::OutOfRange {
        key: key.into(),
        rule: rule.into(),
    }
}

#[cfg(test)]
mod tests;
