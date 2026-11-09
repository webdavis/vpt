use super::{Document, NoteError};
use vpt_domain::note::{MAX_RENDERED, RenderError, managed::Regions};

impl Document<'_> {
    pub fn content_start_line(&self) -> usize {
        let marker = self.source[self.body..]
            .find(vpt_domain::note::managed::CONTENT_START)
            .expect("validated content marker");
        self.source[..self.body + marker]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 2
    }

    pub fn with_open_flags(&self, count: usize) -> Result<String, NoteError> {
        super::rewrite::counts::update(self, count)
    }

    pub fn replace_content(&self, content: &str) -> Result<String, NoteError> {
        self.check_owner(&self.recording, &self.stage)?;
        let regions = Regions::parse(&self.source[self.body..]).map_err(format_error)?;
        let body = regions
            .rewrite(regions.links(), content)
            .map_err(format_error)?;
        if self.body.saturating_add(body.len()) > MAX_RENDERED {
            return Err(NoteError::CapacityExceeded);
        }
        let mut result = String::with_capacity(self.body + body.len());
        result.push_str(&self.source[..self.body]);
        result.push_str(&body);
        Ok(result)
    }
}

fn format_error(error: RenderError) -> NoteError {
    match error {
        RenderError::CapacityExceeded => NoteError::CapacityExceeded,
        _ => NoteError::Markers,
    }
}
