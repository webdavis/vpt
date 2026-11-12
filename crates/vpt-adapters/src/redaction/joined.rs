use super::MAX_TEXT;
use std::ops::Range;
use vpt_domain::redaction::{Error, Redactor};

#[derive(Default)]
pub(super) struct Joined {
    text: String,
    cuts: Vec<usize>,
    starts: Vec<usize>,
    stamps: Vec<Option<String>>,
}

impl Joined {
    pub fn push(
        &mut self,
        source: &str,
        text: &str,
        protected: &[Range<usize>],
    ) -> Result<(), Error> {
        self.starts.push(self.text.len());
        self.stamps.push(stamp(source));
        let mut cursor = 0;
        for range in protected.iter().chain([&(text.len()..text.len())]) {
            if self.text.len() + range.start.saturating_sub(cursor) + 1 > MAX_TEXT {
                return Err(Error::Capacity);
            }
            self.text.push_str(&text[cursor..range.start.max(cursor)]);
            self.cuts.push(self.text.len());
            self.text.push(' ');
            cursor = range.end.max(cursor);
        }
        Ok(())
    }

    pub fn check(&self, redactor: &Redactor) -> Result<(), Error> {
        let Some((class, range)) = redactor.crossing(&self.text, &self.cuts)? else {
            return Ok(());
        };
        let paragraph = |offset: usize| self.starts.partition_point(|start| *start <= offset) - 1;
        let between = [paragraph(range.start), paragraph(range.end - 1)]
            .into_iter()
            .filter_map(|index| self.stamps[index].clone())
            .collect();
        Err(Error::Split { class, between })
    }
}

fn stamp(source: &str) -> Option<String> {
    let rest = &source[source.find('[')? + 1..];
    let time = &rest[..rest.find([']', '-'])?];
    (time.contains(':')
        && time
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ':' | '.')))
    .then(|| time.to_owned())
}
