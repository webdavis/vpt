use super::*;
use crate::note::{MAX_RENDERED, managed::marker};
use std::ops::Range;

pub struct Region<'a> {
    source: &'a str,
    content: Range<usize>,
}

impl<'a> Region<'a> {
    pub fn parse(source: &'a str) -> Result<Self, RenderError> {
        if source.len() > MAX_RENDERED {
            return Err(RenderError::CapacityExceeded);
        }
        let start = marker(source, START)?;
        let end = marker(source, END)?;
        if start.end > end.start {
            return Err(RenderError::Markers);
        }
        Ok(Self {
            source,
            content: start.end..end.start,
        })
    }
    pub fn content(&self) -> &'a str {
        &self.source[self.content.clone()]
    }
    pub fn rewrite(&self, content: &str) -> Result<String, RenderError> {
        if content.len() > MAX_RENDERED {
            return Err(RenderError::CapacityExceeded);
        }
        if content.contains(START) || content.contains(END) {
            return Err(RenderError::Markers);
        }
        let mut output = Writer::default();
        output.literal(&self.source[..self.content.start])?;
        output.literal(content)?;
        if !content.is_empty() && !content.ends_with('\n') {
            output.literal("\n")?;
        }
        output.literal(&self.source[self.content.end..])?;
        Ok(output.finish())
    }
}
