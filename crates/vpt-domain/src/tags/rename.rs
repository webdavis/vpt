use super::{NewTags, Options, Provenance, Tag, Vocabulary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameError {
    Syntax,
    Same,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Rename {
    source: String,
    target: String,
    target_key: String,
}

impl Vocabulary {
    pub fn rename(&self, old: &str, new: &str) -> Result<Rename, RenameError> {
        let source = super::proposed_key(old).map_err(|_| RenameError::Syntax)?;
        let target = self
            .select(
                &[new.to_owned()],
                Options {
                    new_tags: NewTags::Write,
                    max_per_note: 1,
                    max_suggested: 0,
                },
            )
            .ok()
            .and_then(|selected| selected.confirmed.into_iter().next())
            .ok_or(RenameError::Syntax)?;
        let target_key = super::key(&target).map_err(|_| RenameError::Syntax)?;
        if target_key == source {
            return Err(RenameError::Same);
        }
        Ok(Rename {
            source,
            target,
            target_key,
        })
    }
}

impl Rename {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn matches(&self, name: &str) -> bool {
        super::key(name).is_ok_and(|key| key == self.source)
    }

    pub fn is_target(&self, name: &str) -> bool {
        super::key(name).is_ok_and(|key| key == self.target_key)
    }

    pub fn apply(&self, tags: &[Tag]) -> Option<Vec<Tag>> {
        let from = tags.iter().position(|tag| self.matches(&tag.name))?;
        let mut tags = tags.to_vec();
        let Some(to) = tags.iter().position(|tag| self.is_target(&tag.name)) else {
            tags[from].name = self.target.clone();
            return Some(tags);
        };
        let hand_set_old = tags[from].provenance == Provenance::Manual
            && tags[to].provenance != Provenance::Manual;
        let kept = if hand_set_old {
            Tag {
                name: tags[to].name.clone(),
                ..tags[from].clone()
            }
        } else {
            tags[to].clone()
        };
        tags[from.min(to)] = kept;
        tags.remove(from.max(to));
        Some(tags)
    }
}

impl std::fmt::Debug for Rename {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Rename")
    }
}

#[cfg(test)]
mod tests;
