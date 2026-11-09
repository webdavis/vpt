use crate::normalize::{self, AlignmentTokens};

mod myers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    Equal { main: usize, checker: usize },
    Delete { main: usize },
    Insert { checker: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Aligned { distance: usize, edits: Vec<Edit> },
    Divergent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub main: AlignmentTokens,
    pub checker: AlignmentTokens,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentError {
    InvalidRatio,
    CapacityExceeded,
}

pub fn compare(
    main: &[&str],
    checker: &[&str],
    max_ratio: f64,
) -> Result<Comparison, AlignmentError> {
    if !max_ratio.is_finite() || max_ratio <= 0.0 || max_ratio > 1.0 {
        return Err(AlignmentError::InvalidRatio);
    }
    if [main, checker].iter().any(|words| {
        words.len() > 65_536
            || words.iter().map(|word| word.len()).sum::<usize>() > 16 * 1024 * 1024
    }) {
        return Err(AlignmentError::CapacityExceeded);
    }
    let main = normalize::alignment(main);
    let checker = normalize::alignment(checker);
    let count = main.tokens.len().max(checker.tokens.len());
    let mut limit = (max_ratio * count as f64).floor() as usize;
    if count > 0 && (limit + 1) as f64 / count as f64 <= max_ratio {
        limit += 1;
    }
    let outcome = myers::diff(&main.tokens, &checker.tokens, limit)?;
    Ok(Comparison {
        main,
        checker,
        outcome,
    })
}

#[cfg(test)]
mod tests;
