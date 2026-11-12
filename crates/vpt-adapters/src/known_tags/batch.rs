use super::*;
use std::collections::HashSet;

pub struct Edit {
    pub update: Update,
    pub appended: Vec<String>,
    pub keys: HashSet<String>,
}

/// Removes every line whose tag key is in `drop`, then appends the tags not yet present.
/// `appended` holds the keys this edit added and `keys` every key the file holds afterwards.
pub fn prepare_many(
    path: &Path,
    tags: &[String],
    drop: &HashSet<String>,
) -> Result<Edit, TermsError> {
    if tags.len() > 256 {
        return Err(TermsError::TooLarge);
    }
    Vocabulary::new(tags).map_err(error)?;
    let mut appended = Vec::new();
    let mut keys = HashSet::new();
    let update = known_terms::prepare_with(path, |bytes| {
        known_terms::document::lines(bytes)?;
        let text = std::str::from_utf8(bytes).map_err(|_| TermsError::InvalidText)?;
        let mut output = Vec::with_capacity(bytes.len());
        let mut count = 0;
        for line in text.split_inclusive('\n') {
            let tag = line.trim();
            if !tag.is_empty() {
                let key = vpt_domain::tags::key(tag).map_err(error)?;
                if drop.contains(&key) {
                    continue;
                }
                keys.insert(key);
                count += 1;
            }
            output.extend_from_slice(line.as_bytes());
        }
        for tag in tags {
            let key = vpt_domain::tags::key(tag).map_err(error)?;
            if !keys.insert(key.clone()) {
                continue;
            }
            let separator = !output.is_empty() && !output.ends_with(b"\n");
            if count == known_terms::MAX_TERMS
                || output.len() + usize::from(separator) + tag.len() + 1 > known_terms::MAX_BYTES
            {
                return Err(TermsError::TooLarge);
            }
            if separator {
                output.push(b'\n');
            }
            output.extend_from_slice(tag.as_bytes());
            output.push(b'\n');
            appended.push(key);
            count += 1;
        }
        Ok(output)
    })?;
    Ok(Edit {
        update,
        appended,
        keys,
    })
}
