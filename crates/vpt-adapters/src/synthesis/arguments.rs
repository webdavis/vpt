use vpt_application::{
    ports::{SynthesisFailure, SynthesisRequest},
    tag_hints::{KNOWN_TAGS, REFUSED_TAGS, TagHints},
};

const LIMIT: usize = 1_048_576;
const KNOWN_TAGS_SHOWN: usize = 300;
const TAG_LIST_BYTES: usize = 65_536;

pub(super) fn argv(request: &SynthesisRequest<'_>) -> Result<Vec<String>, SynthesisFailure> {
    let settings = request.settings;
    if settings.command.is_empty() {
        return Err(SynthesisFailure::Disabled);
    }
    if !crate::arguments::valid(&settings.command, LIMIT)
        || settings.timeout.is_zero()
        || request.language.trim().is_empty()
    {
        return Err(SynthesisFailure::InvalidArguments);
    }
    let base = expand(request, "", "")?;
    let placeholders: usize = settings
        .command
        .iter()
        .map(|argument| {
            argument.matches(KNOWN_TAGS).count() + argument.matches(REFUSED_TAGS).count()
        })
        .sum();
    if placeholders == 0 {
        return Ok(base);
    }
    let spare = LIMIT - base.iter().map(String::len).sum::<usize>();
    let (known, refused) = lists(request.tags, (spare / placeholders).min(TAG_LIST_BYTES));
    expand(request, &known, &refused)
}

fn expand(
    request: &SynthesisRequest<'_>,
    known: &str,
    refused: &str,
) -> Result<Vec<String>, SynthesisFailure> {
    let path = request
        .transcript
        .to_str()
        .filter(|_| request.transcript.is_absolute())
        .ok_or(SynthesisFailure::InvalidArguments)?;
    let captured = request.captured.0.rfc3339_with(request.captured.1);
    let values = [
        ("{transcript}", path),
        ("{id}", request.recording.as_str()),
        ("{language}", request.language),
        ("{captured_at}", &captured),
        (KNOWN_TAGS, known),
        (REFUSED_TAGS, refused),
    ];
    if values.iter().any(|(_, value)| value.contains('\0')) {
        return Err(SynthesisFailure::InvalidArguments);
    }
    let mut remaining = LIMIT;
    let argv: Vec<_> = request
        .settings
        .command
        .iter()
        .map(|argument| crate::arguments::replace(argument, &values, &mut remaining))
        .collect::<Option<_>>()
        .ok_or(SynthesisFailure::InvalidArguments)?;
    if !crate::arguments::valid(&argv, LIMIT) {
        return Err(SynthesisFailure::InvalidArguments);
    }
    Ok(argv)
}

fn lists(tags: &TagHints, bytes: usize) -> (String, String) {
    let known = tags
        .known
        .iter()
        .take(KNOWN_TAGS_SHOWN)
        .map(|tag| format!("{} ({})", tag.name, tag.notes));
    let refused = tags.refused.iter().map(|tag| match &tag.renamed_to {
        Some(new) => format!("{} -> {new}", tag.name),
        None => tag.name.clone(),
    });
    (join(known, bytes), join(refused, bytes))
}

fn join(items: impl Iterator<Item = String>, bytes: usize) -> String {
    let mut text = String::new();
    for item in items {
        let separator = if text.is_empty() { "" } else { ", " };
        if text.len() + separator.len() + item.len() > bytes {
            break;
        }
        text.push_str(separator);
        text.push_str(&item);
    }
    text
}
