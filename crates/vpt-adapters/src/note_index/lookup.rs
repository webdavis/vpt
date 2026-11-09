use super::*;
use std::collections::HashSet;

impl Index {
    pub fn matching(&self, name: &str) -> &[PathBuf] {
        let Some(key) = key(name) else {
            return &[];
        };
        self.names.get(&key).map_or(&[], Vec::as_slice)
    }

    pub(super) fn build_lookup(&mut self, remaining: &mut usize) -> Result<(), StoreError> {
        for note in &self.notes {
            let mut seen = HashSet::new();
            for name in std::iter::once(&note.name).chain(&note.metadata.aliases) {
                let key = key(name).ok_or_else(capacity)?;
                if key.is_empty() || !seen.insert(key.clone()) {
                    continue;
                }
                *remaining = remaining
                    .checked_sub(
                        key.len()
                            + note.path.as_os_str().len()
                            + std::mem::size_of::<(String, Vec<PathBuf>, PathBuf)>(),
                    )
                    .ok_or_else(capacity)?;
                self.names.entry(key).or_default().push(note.path.clone());
            }
        }
        Ok(())
    }
}

fn key(text: &str) -> Option<String> {
    if text.len() > 4 * 65_536 || text.chars().count() > 65_536 {
        return None;
    }
    let canonical = vpt_domain::normalize::canonical(text);
    if canonical.len() > 4 * 65_536 || canonical.chars().count() > 65_536 {
        return None;
    }
    Some(canonical.split_whitespace().collect::<Vec<_>>().join(" "))
}
