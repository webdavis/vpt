use super::*;
pub(super) struct Edit {
    pub range: Range<usize>,
    pub text: String,
    pub marker: Option<Range<usize>>,
}
struct Piece {
    output: Range<usize>,
    source: Range<usize>,
    exact: bool,
}
pub(super) struct Mapped {
    pub text: String,
    pub markers: Vec<Range<usize>>,
    pieces: Vec<Piece>,
    original_len: usize,
}
impl Mapped {
    pub fn apply(source: &str, mut edits: Vec<Edit>) -> Result<Self, Error> {
        if source.len() > vpt_domain::note::MAX_RENDERED || edits.len() > MAX_ITEMS * 2 {
            return Err(Error::Capacity);
        }
        edits.sort_by_key(|e| (e.range.start, e.range.end));
        let mut result = Self {
            text: String::new(),
            markers: vec![],
            pieces: vec![],
            original_len: source.len(),
        };
        let mut cursor = 0;
        for edit in edits {
            if edit.range.start < cursor
                || edit.range.start > edit.range.end
                || edit.range.end > source.len()
                || !source.is_char_boundary(edit.range.start)
                || !source.is_char_boundary(edit.range.end)
            {
                return Err(Error::InvalidText);
            }
            result.push(
                &source[cursor..edit.range.start],
                cursor..edit.range.start,
                true,
            )?;
            let start = result.text.len();
            if let Some(marker) = edit.marker {
                if marker.start >= marker.end || marker.end > edit.text.len() {
                    return Err(Error::InvalidText);
                }
                result
                    .markers
                    .push(start + marker.start..start + marker.end);
            }
            result.push(&edit.text, edit.range.clone(), false)?;
            cursor = edit.range.end;
        }
        result.push(&source[cursor..], cursor..source.len(), true)?;
        Ok(result)
    }
    fn push(&mut self, text: &str, source: Range<usize>, exact: bool) -> Result<(), Error> {
        if text.len() > vpt_domain::note::MAX_RENDERED - self.text.len() {
            return Err(Error::Capacity);
        }
        if !text.is_empty() {
            let start = self.text.len();
            self.text.push_str(text);
            self.pieces.push(Piece {
                output: start..self.text.len(),
                source,
                exact,
            });
        }
        Ok(())
    }
    pub fn original(&self, range: Range<usize>) -> Result<Range<usize>, Error> {
        if range.start > range.end || range.end > self.text.len() {
            return Err(Error::InvalidText);
        }
        let first = self
            .pieces
            .get(self.pieces.partition_point(|p| p.output.end <= range.start));
        let start = first.map_or(self.original_len, |p| {
            if p.exact {
                p.source.start + range.start - p.output.start
            } else {
                p.source.start
            }
        });
        if range.is_empty() {
            return Ok(start..start);
        }
        let last = self
            .pieces
            .get(self.pieces.partition_point(|p| p.output.end < range.end))
            .ok_or(Error::InvalidText)?;
        let end = if last.exact {
            last.source.start + range.end - last.output.start
        } else {
            last.source.end
        };
        Ok(start..end)
    }
}
