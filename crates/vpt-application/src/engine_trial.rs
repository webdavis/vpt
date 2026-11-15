use std::collections::BTreeSet;

pub const SENTENCE: &str = "The purple kettle sang seven songs on Tuesday morning. A paper boat \
                            drifted past the quiet lighthouse. Nobody counted the yellow pebbles \
                            twice.";

pub const LANGUAGE: &str = "en-US";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heard {
    pub found: usize,
    pub expected: usize,
}

impl Heard {
    pub fn enough(self) -> bool {
        self.found * 3 >= self.expected * 2
    }
}

pub fn heard(transcript: &str) -> Heard {
    let expected = words(SENTENCE);
    Heard {
        found: expected.intersection(&words(transcript)).count(),
        expected: expected.len(),
    }
}

fn words(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests;
