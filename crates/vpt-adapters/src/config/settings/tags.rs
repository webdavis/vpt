use super::*;
use vpt_application::TagSettings;
use vpt_domain::tags::{NewTags, Options};

pub(super) fn from_table(table: &Table, home: &Path) -> Result<TagSettings, ConfigError> {
    let new_tags = match text(table, "tags.new_tags")? {
        "hold" => NewTags::Hold,
        "write" => NewTags::Write,
        _ => {
            return Err(ConfigError::OutOfRange {
                key: "tags.new_tags".into(),
                rule: "one of hold, write".into(),
            });
        }
    };
    Ok(TagSettings {
        options: Options {
            new_tags,
            max_per_note: small(table, "tags.max_per_note")?,
            max_suggested: small(table, "tags.max_suggested")?,
        },
        known_tags_path: expand(text(table, "tags.known_tags_path")?, home, None),
    })
}
