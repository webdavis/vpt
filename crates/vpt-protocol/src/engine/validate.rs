use super::{EngineDecodeError, EngineDocument, ExpectedEngine};
use EngineDecodeError::Invalid;

pub(crate) fn validate(
    value: &EngineDocument,
    expected: &ExpectedEngine,
    duration: f64,
) -> Result<(), EngineDecodeError> {
    if !duration.is_finite() || duration < 0.0 {
        return Err(Invalid("duration"));
    }
    if value.engine.name != expected.name
        || value.engine.family != expected.family
        || value.local != expected.local
    {
        return Err(Invalid("engine identity"));
    }
    for field in [
        &value.engine.name,
        &value.engine.tool,
        &value.engine.version,
        &value.engine.model,
        &value.engine.family,
    ] {
        if field.trim().is_empty() {
            return Err(Invalid("engine metadata"));
        }
    }
    if value.language.trim().is_empty() {
        return Err(Invalid("language"));
    }
    if value
        .segments
        .iter()
        .all(|segment| segment.text.trim().is_empty())
        || value
            .segments
            .iter()
            .all(|segment| segment.words.is_empty())
    {
        return Err(Invalid("empty transcript"));
    }
    let mut previous_segment = 0.0;
    for segment in &value.segments {
        if !range(segment.start, segment.end, 0.0, duration) {
            return Err(Invalid("segment range"));
        }
        if segment.start < previous_segment {
            return Err(Invalid("segment order"));
        }
        previous_segment = segment.start;
        if segment
            .speaker
            .as_ref()
            .is_some_and(|speaker| speaker.trim().is_empty())
        {
            return Err(Invalid("speaker"));
        }
        let mut previous_word = segment.start;
        for word in &segment.words {
            if word.text.trim().is_empty() {
                return Err(Invalid("word text"));
            }
            if !range(word.start, word.end, segment.start, segment.end) {
                return Err(Invalid("word range"));
            }
            if word.start < previous_word {
                return Err(Invalid("word order"));
            }
            previous_word = word.start;
            if word
                .confidence
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            {
                return Err(Invalid("confidence"));
            }
        }
    }
    Ok(())
}

fn range(start: f64, end: f64, lower: f64, upper: f64) -> bool {
    start.is_finite() && end.is_finite() && lower <= start && start <= end && end <= upper
}
