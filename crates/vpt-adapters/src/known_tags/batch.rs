use super::*;
use std::collections::HashSet;

pub fn prepare_many(path: &Path, tags: &[String]) -> Result<Update, TermsError> {
    if tags.len() > 256 {
        return Err(TermsError::TooLarge);
    }
    Vocabulary::new(tags).map_err(error)?;
    known_terms::prepare_with(path, |bytes| {
        let existing = known_terms::document::lines(bytes)?;
        let mut keys: HashSet<_> = existing
            .iter()
            .map(|tag| vpt_domain::tags::key(tag).map_err(error))
            .collect::<Result<_, _>>()?;
        let mut count = existing.len();
        let mut output = bytes.to_vec();
        for tag in tags {
            if !keys.insert(vpt_domain::tags::key(tag).map_err(error)?) {
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
            count += 1;
        }
        Ok(output)
    })
}
