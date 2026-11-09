use super::*;
#[derive(Clone, PartialEq, Eq)]
pub struct Candidate {
    pub token: String,
    pub line: usize,
}
impl std::fmt::Debug for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Candidate")
            .field("line", &self.line)
            .finish_non_exhaustive()
    }
}
pub(super) fn body(
    output: &mut Vec<Candidate>,
    masked: &vpt_domain::redaction::MaskedText,
    mut line: usize,
) -> Result<(), Error> {
    let mut cursor = masked.text.len() - masked.text.trim_start().len();
    for candidate in vpt_domain::redaction::candidates(&masked.text, &masked.masks)? {
        line += masked.text[cursor..candidate.bytes.start]
            .bytes()
            .filter(|c| *c == b'\n')
            .count();
        cursor = candidate.bytes.start;
        push(output, candidate.token, line)?;
    }
    Ok(())
}
pub(super) fn header(output: &mut Vec<Candidate>, text: &str, line: usize) -> Result<(), Error> {
    for candidate in vpt_domain::redaction::candidates(text, &[])? {
        push(output, candidate.token, line)?;
    }
    Ok(())
}
fn push(output: &mut Vec<Candidate>, token: String, line: usize) -> Result<(), Error> {
    if output.len() >= MAX_ITEMS {
        return Err(Error::Capacity);
    }
    output.push(Candidate { token, line });
    Ok(())
}
#[cfg(test)]
mod tests;
