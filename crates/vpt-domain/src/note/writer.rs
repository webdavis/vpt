use super::{MAX_RENDERED, RenderError};

#[derive(Default)]
pub(crate) struct Writer(String);

impl Writer {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn literal(&mut self, text: &str) -> Result<(), RenderError> {
        if text.len() > MAX_RENDERED - self.0.len() {
            return Err(RenderError::CapacityExceeded);
        }
        self.0.push_str(text);
        Ok(())
    }

    pub fn text(&mut self, text: &str) -> Result<(), RenderError> {
        for (offset, ch) in text.char_indices() {
            match ch {
                '&' => self.literal("&amp;")?,
                '<' => self.literal("&lt;")?,
                '>' => self.literal("&gt;")?,
                '[' | ']' | '\\' | '`' | '*' | '_' | '{' | '}' | '#' | '|' | '~' | ':' | '@'
                | '%' | '=' | '^' | '$' => self.literal(&format!("&#{};", ch as u32))?,
                '.' if text
                    .get(offset.saturating_sub(3)..offset)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www")) =>
                {
                    self.literal("&#46;")?
                }
                ch if ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}') => {
                    self.literal(&format!("&#{};", ch as u32))?
                }
                _ => self.literal(ch.encode_utf8(&mut [0; 4]))?,
            }
        }
        Ok(())
    }

    pub fn finish(self) -> String {
        self.0
    }
}
