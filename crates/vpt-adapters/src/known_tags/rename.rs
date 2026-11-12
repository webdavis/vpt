use super::*;
use vpt_domain::tags::Rename;

pub fn prepare_renamed(path: &Path, rename: &Rename) -> Result<Update, TermsError> {
    known_terms::prepare_with(path, |bytes| {
        known_terms::document::lines(bytes)?;
        let text = std::str::from_utf8(bytes).map_err(|_| TermsError::InvalidText)?;
        if !text.lines().any(|line| rename.matches(line.trim())) {
            return Ok(bytes.to_vec());
        }
        let mut placed = text.lines().any(|line| rename.is_target(line.trim()));
        let mut output = String::with_capacity(bytes.len());
        for line in text.split_inclusive('\n') {
            if !rename.matches(line.trim()) {
                output.push_str(line);
            } else if !placed {
                output.push_str(rename.target());
                output.push('\n');
                placed = true;
            }
        }
        if output.len() > known_terms::MAX_BYTES {
            return Err(TermsError::TooLarge);
        }
        Ok(output.into_bytes())
    })
}
