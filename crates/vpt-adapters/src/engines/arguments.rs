use vpt_application::ports::{EngineFailure, EngineRequest};

pub(super) fn substitute(
    command: &[String],
    request: &EngineRequest<'_>,
) -> Result<Vec<String>, EngineFailure> {
    let audio = request
        .audio
        .to_str()
        .ok_or(EngineFailure::InvalidArguments)?;
    let out_dir = request
        .out_dir
        .to_str()
        .ok_or(EngineFailure::InvalidArguments)?;
    let values = [
        ("{audio}", audio),
        ("{language}", request.language),
        ("{out_dir}", out_dir),
    ];
    let mut remaining = super::DOCUMENT_LIMIT;
    let argv: Vec<_> = command
        .iter()
        .map(|argument| crate::arguments::replace(argument, &values, &mut remaining))
        .collect::<Option<_>>()
        .ok_or(EngineFailure::InvalidArguments)?;
    if argv.first().is_none_or(|program| program.trim().is_empty())
        || argv.iter().any(|argument| argument.contains('\0'))
    {
        return Err(EngineFailure::InvalidArguments);
    }
    Ok(argv)
}
