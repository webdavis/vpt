use super::{Segment, Transcript};

pub fn label_speakers(record: &Transcript, checker: Option<&Transcript>) -> Transcript {
    let labels: Vec<Option<&str>> = match checker {
        _ if record
            .segments
            .iter()
            .any(|segment| segment.speaker.is_some()) =>
        {
            record
                .segments
                .iter()
                .map(|segment| segment.speaker.as_deref())
                .collect()
        }
        Some(checker) => record
            .segments
            .iter()
            .map(|segment| longest_overlap(segment, checker))
            .collect(),
        None => vec![None; record.segments.len()],
    };
    let mut order: Vec<&str> = Vec::new();
    for label in labels.iter().flatten() {
        if !order.contains(label) {
            order.push(label);
        }
    }
    let mut labeled = record.clone();
    for (segment, label) in labeled.segments.iter_mut().zip(labels) {
        segment.speaker = label.filter(|_| order.len() > 1).and_then(|label| {
            let number = order.iter().position(|known| *known == label)?;
            Some(format!("Speaker {}", number + 1))
        });
    }
    labeled
}

fn longest_overlap<'a>(segment: &Segment, checker: &'a Transcript) -> Option<&'a str> {
    let mut totals: Vec<(&str, f64)> = Vec::new();
    for other in &checker.segments {
        let Some(label) = other.speaker.as_deref() else {
            continue;
        };
        let overlap = segment.end.min(other.end) - segment.start.max(other.start);
        if overlap <= 0.0 {
            continue;
        }
        match totals.iter_mut().find(|(known, _)| *known == label) {
            Some((_, total)) => *total += overlap,
            None => totals.push((label, overlap)),
        }
    }
    let mut best: Option<(&str, f64)> = None;
    for (label, total) in totals {
        if best.is_none_or(|(_, longest)| total > longest) {
            best = Some((label, total));
        }
    }
    best.map(|(label, _)| label)
}
