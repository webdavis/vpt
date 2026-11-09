use super::SourceRange;

pub(super) fn resolve(references: &[String], duration: f64) -> Option<Vec<SourceRange>> {
    references
        .iter()
        .map(|reference| {
            let source = SourceRange::parse(reference)?;
            (source.end_secs as f64 <= duration).then_some(source)
        })
        .collect()
}

impl SourceRange {
    pub fn parse(reference: &str) -> Option<Self> {
        if reference.len() > 64 {
            return None;
        }
        let (start, end) = reference.split_once('-')?;
        let start_secs = seconds(start)?;
        let end_secs = seconds(end)?;
        (start_secs < end_secs && end_secs <= 9_007_199_254_740_992).then_some(Self {
            start_secs,
            end_secs,
        })
    }
}

fn seconds(text: &str) -> Option<u64> {
    let (minutes, seconds) = text.split_once(':')?;
    if minutes.len() < 2
        || seconds.len() != 2
        || !minutes.bytes().all(|b| b.is_ascii_digit())
        || !seconds.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let seconds = seconds.parse::<u64>().ok()?;
    if seconds >= 60 {
        return None;
    }
    minutes
        .parse::<u64>()
        .ok()?
        .checked_mul(60)?
        .checked_add(seconds)
}

pub(super) fn intersects(start: f64, end: f64, source_start: f64, source_end: f64) -> bool {
    if start == end {
        source_start <= start && start < source_end
    } else {
        start < source_end && end > source_start
    }
}
