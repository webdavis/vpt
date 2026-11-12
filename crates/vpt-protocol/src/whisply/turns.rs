use super::model::{Chunk, Word};
use crate::engine::{self, Segment};

pub(super) fn segments(chunk: Chunk) -> Vec<Segment> {
    let mut turns: Vec<Vec<Word>> = Vec::new();
    for word in chunk.words {
        match turns.last_mut() {
            Some(turn) if speaker(&turn[0]) == speaker(&word) => turn.push(word),
            _ => turns.push(vec![word]),
        }
    }
    if turns.len() < 2 {
        let words = turns.pop().unwrap_or_default();
        let speaker = words.first().and_then(speaker);
        return vec![segment(chunk.timestamp, chunk.text, speaker, words)];
    }
    turns
        .into_iter()
        .map(|words| {
            let text: Vec<_> = words.iter().map(|word| word.word.as_str()).collect();
            let range = [words[0].start, words[words.len() - 1].end];
            segment(range, text.join(" "), speaker(&words[0]), words)
        })
        .collect()
}

fn speaker(word: &Word) -> Option<String> {
    word.speaker.clone().filter(|speaker| speaker != "UNKNOWN")
}

fn segment(range: [f64; 2], text: String, speaker: Option<String>, words: Vec<Word>) -> Segment {
    Segment {
        start: range[0],
        end: range[1],
        text,
        words: words
            .into_iter()
            .map(|word| engine::Word {
                text: word.word,
                start: word.start,
                end: word.end,
                confidence: word.score,
            })
            .collect(),
        speaker,
    }
}
