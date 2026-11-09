use std::ops::Range;
mod fillers;
mod transform;

#[derive(Clone, PartialEq, Eq)]
pub struct Readable {
    pub text: String,
    pub protected: Vec<Range<usize>>,
}

impl std::fmt::Debug for Readable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Readable")
            .field("protected", &self.protected)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadabilityError {
    InvalidRanges,
    CapacityExceeded,
}

pub fn apply(
    text: &str,
    protected: &[Range<usize>],
    enabled: bool,
) -> Result<Readable, ReadabilityError> {
    if text.len() > 16 * 1024 * 1024 || protected.len() > 65_536 {
        return Err(ReadabilityError::CapacityExceeded);
    }
    let mut previous_end = 0;
    for range in protected {
        if range.start < previous_end
            || range.start > range.end
            || range.end > text.len()
            || !text.is_char_boundary(range.start)
            || !text.is_char_boundary(range.end)
        {
            return Err(ReadabilityError::InvalidRanges);
        }
        previous_end = range.end;
    }
    if !enabled {
        return Ok(Readable {
            text: text.into(),
            protected: protected.to_vec(),
        });
    }
    let deleted = fillers::deletions(text, protected);
    Ok(transform::apply(text, protected, &deleted))
}

fn overlaps(ranges: &[Range<usize>], start: usize, end: usize) -> bool {
    ranges[ranges.partition_point(|range| range.end <= start)..]
        .iter()
        .take_while(|range| range.start < end)
        .any(|range| !range.is_empty())
}

#[cfg(test)]
mod tests;
