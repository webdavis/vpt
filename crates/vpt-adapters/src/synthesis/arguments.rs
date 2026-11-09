use vpt_application::ports::{SynthesisFailure, SynthesisRequest};

pub(super) fn argv(request: &SynthesisRequest<'_>) -> Result<Vec<String>, SynthesisFailure> {
    let settings = request.settings;
    if settings.command.is_empty() {
        return Err(SynthesisFailure::Disabled);
    }
    if !crate::arguments::valid(&settings.command, 1_048_576)
        || settings.timeout.is_zero()
        || request.language.trim().is_empty()
    {
        return Err(SynthesisFailure::InvalidArguments);
    }
    let path = request
        .transcript
        .to_str()
        .filter(|_| request.transcript.is_absolute())
        .ok_or(SynthesisFailure::InvalidArguments)?;
    let values = [
        ("{transcript}", path),
        ("{id}", request.recording.as_str()),
        ("{language}", request.language),
    ];
    if values.iter().any(|(_, value)| value.contains('\0')) {
        return Err(SynthesisFailure::InvalidArguments);
    }
    let mut remaining = 1_048_576;
    let argv: Vec<_> = settings
        .command
        .iter()
        .map(|argument| crate::arguments::replace(argument, &values, &mut remaining))
        .collect::<Option<_>>()
        .ok_or(SynthesisFailure::InvalidArguments)?;
    if !crate::arguments::valid(&argv, 1_048_576) {
        return Err(SynthesisFailure::InvalidArguments);
    }
    Ok(argv)
}
