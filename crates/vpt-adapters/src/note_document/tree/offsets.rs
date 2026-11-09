pub(super) struct Offsets(Vec<(usize, usize)>);

pub(super) fn container_end(
    source: &str,
    span: &std::ops::Range<usize>,
) -> Result<usize, super::NoteError> {
    match source.as_bytes().get(span.start) {
        Some(b'}' | b']') => return Ok(span.start + 1),
        Some(b',') => {}
        _ => return Ok(span.end),
    }
    // In 0.0.11, a flow mapping ending with a comma reports that comma's span.
    // The parser has already accepted the closing brace after whitespace/comments.
    let mut comment = false;
    let start = span.start + 1;
    for (offset, ch) in source[start..].char_indices() {
        if matches!(ch, '\r' | '\n') {
            comment = false;
            continue;
        }
        if ch == '#' {
            comment = true;
        }
        if comment || ch.is_whitespace() {
            continue;
        }
        return if ch == '}' {
            Ok(start + offset + 1)
        } else {
            Err(super::NoteError::Frontmatter)
        };
    }
    Err(super::NoteError::Frontmatter)
}

impl Offsets {
    pub fn new(source: &str) -> Self {
        let mut changes = Vec::new();
        let mut extra = 0;
        for (index, ch) in source.chars().enumerate() {
            if ch.len_utf8() > 1 {
                extra += ch.len_utf8() - 1;
                changes.push((index + 1, extra));
            }
        }
        Self(changes)
    }

    pub fn byte(&self, character: usize) -> usize {
        // saphyr-parser 0.0.11 markers count Unicode scalars, including in StrInput.
        let count = self.0.partition_point(|(index, _)| *index <= character);
        character + count.checked_sub(1).map_or(0, |index| self.0[index].1)
    }
}
