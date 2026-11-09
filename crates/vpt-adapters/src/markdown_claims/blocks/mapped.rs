use super::*;

pub(in crate::markdown_claims) struct Block {
    pub text: String,
    pub end: usize,
    pub list: bool,
    pieces: Vec<Piece>,
}
struct Piece {
    text: Range<usize>,
    source: Range<usize>,
    exact: bool,
    plain: bool,
}

impl Block {
    pub(super) fn new(start: usize, list: bool) -> Self {
        Self {
            text: String::new(),
            end: start,
            list,
            pieces: vec![],
        }
    }
    pub(super) fn push(
        &mut self,
        source: &str,
        text: &str,
        range: Range<usize>,
        plain: bool,
    ) -> Result<(), ClaimError> {
        if self.text.len().saturating_add(text.len()) > MAX_TEXT
            || self.pieces.len() >= MAX_ITEMS * 4
        {
            return Err(ClaimError::CapacityExceeded);
        }
        if text.is_empty() {
            return Ok(());
        }
        let start = self.text.len();
        self.text.push_str(text);
        self.end = range.end;
        self.pieces.push(Piece {
            text: start..self.text.len(),
            exact: source[range.clone()] == *text,
            source: range,
            plain,
        });
        Ok(())
    }
    fn piece(&self, offset: usize) -> &Piece {
        &self.pieces[self
            .pieces
            .partition_point(|piece| piece.text.end <= offset)]
    }
    pub(in crate::markdown_claims) fn plain(&self, offset: usize) -> bool {
        self.piece(offset).plain
    }
    pub(in crate::markdown_claims) fn syntax(&self, offset: usize) -> bool {
        let piece = self.piece(offset);
        piece.plain && piece.exact
    }
    pub(in crate::markdown_claims) fn start(&self, offset: usize) -> usize {
        let piece = self.piece(offset);
        if piece.exact {
            piece.source.start + offset - piece.text.start
        } else {
            piece.source.start
        }
    }
    pub(in crate::markdown_claims) fn finish(&self, offset: usize) -> usize {
        if offset == self.text.len() {
            return self.end;
        }
        let piece = self.piece(offset - 1);
        if piece.exact {
            piece.source.start + offset - piece.text.start
        } else {
            piece.source.end
        }
    }
    pub(in crate::markdown_claims) fn raw(&self, range: Range<usize>) -> Range<usize> {
        let piece = self.piece(range.end - 1);
        let end = if piece.exact {
            piece.source.start + range.end - piece.text.start
        } else {
            piece.source.end
        };
        self.start(range.start)..end
    }
}
