use crate::{grounding::SourceRange, note::NoteStage};

pub struct Query(Vec<String>);

impl Query {
    pub fn new(words: &[String]) -> Option<Self> {
        (!words.is_empty()).then(|| Self(words.iter().map(|word| word.to_lowercase()).collect()))
    }

    pub fn matches(&self, text: &str) -> bool {
        let text = text.to_lowercase();
        self.0.iter().all(|word| text.contains(word.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moment {
    pub seconds: Option<u64>,
    pub quote: String,
}

/// Takes a note's managed content region, not the whole file.
pub fn moments(stage: NoteStage, content: &str) -> impl Iterator<Item = Moment> + '_ {
    content.lines().filter_map(move |line| match stage {
        NoteStage::Transcript => segment(line),
        NoteStage::Analysis => claim(line),
    })
}

pub fn timecode(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn segment(line: &str) -> Option<Moment> {
    let (time, text) = line.strip_prefix('[')?.split_once("] ")?;
    Some(Moment {
        seconds: Some(crate::grounding::timecode_seconds(time)?),
        quote: unescape(text),
    })
}

fn claim(line: &str) -> Option<Moment> {
    let line = line.strip_prefix("- ")?;
    let (text, sources) = line.split_once(" [").unwrap_or((line, ""));
    let seconds = sources
        .split(['[', ']'])
        .find_map(SourceRange::parse)
        .map(|range| range.start_secs);
    Some(Moment {
        seconds,
        quote: unescape(text),
    })
}

/// Reverses the note writer's character references so a quote reads as it was spoken.
fn unescape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        output.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest
            .find(';')
            .and_then(|end| Some((reference(&rest[1..end])?, end)));
        match decoded {
            Some((character, end)) => {
                output.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                output.push('&');
                rest = &rest[1..];
            }
        }
    }
    output.push_str(rest);
    output
}

fn reference(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        _ => char::from_u32(name.strip_prefix('#')?.parse().ok()?),
    }
}

#[cfg(test)]
mod tests;
