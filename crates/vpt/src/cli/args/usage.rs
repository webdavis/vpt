use super::Invocation;
use clap::error::{ContextKind, ContextValue, ErrorKind};

const FORMATS: [&str; 2] = ["--json", "--toon"];

#[derive(Debug, PartialEq, Eq)]
pub struct UsageError {
    pub message: String,
    pub tips: Vec<String>,
}

impl UsageError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tips: vec![],
        }
    }
}

/// Describes a clap error from vpt's own names alone, so nothing the user typed is repeated.
pub(super) fn from_clap(error: &clap::Error) -> Result<Invocation, UsageError> {
    let named = |kind| defined(error, kind, "', '");
    let message = match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            return Ok(Invocation::print(error.render().to_string()));
        }
        ErrorKind::ValueValidation | ErrorKind::InvalidValue => return Err(without_value(error)),
        ErrorKind::UnknownArgument => match error.get(ContextKind::InvalidArg) {
            Some(ContextValue::String(typed)) if !typed.starts_with('-') => {
                "unexpected extra argument".into()
            }
            Some(ContextValue::String(typed)) if FORMATS.contains(&typed.as_str()) => {
                let format = typed.trim_start_matches('-');
                format!("'--{format}' is not an option; choose the format with '-o {format}'")
            }
            _ => {
                "unknown flag, or a value that starts with '-'; put '--' before such a value".into()
            }
        },
        ErrorKind::InvalidSubcommand => "unknown command".into(),
        ErrorKind::TooManyValues => format!("'{}' takes no value", named(ContextKind::InvalidArg)),
        ErrorKind::ArgumentConflict
            if error.get(ContextKind::InvalidArg) == error.get(ContextKind::PriorArg) =>
        {
            format!(
                "'{}' is given more than once",
                named(ContextKind::InvalidArg)
            )
        }
        ErrorKind::ArgumentConflict => format!(
            "'{}' cannot be used with '{}'",
            named(ContextKind::InvalidArg),
            named(ContextKind::PriorArg)
        ),
        ErrorKind::MissingRequiredArgument => format!(
            "{} is required",
            defined(error, ContextKind::InvalidArg, " and ")
        ),
        ErrorKind::MissingSubcommand => format!(
            "'{}' needs a subcommand",
            named(ContextKind::InvalidSubcommand)
        ),
        ErrorKind::TooFewValues | ErrorKind::WrongNumberOfValues => {
            format!(
                "wrong number of values for '{}'",
                named(ContextKind::InvalidArg)
            )
        }
        kind => kind.as_str().unwrap_or("invalid arguments").into(),
    };
    Err(UsageError {
        message,
        tips: suggestions(error),
    })
}

fn defined(error: &clap::Error, kind: ContextKind, separator: &str) -> String {
    match error.get(kind) {
        Some(ContextValue::String(name)) => name.clone(),
        Some(ContextValue::Strings(names)) => names.join(separator),
        _ => String::new(),
    }
}

fn suggestions(error: &clap::Error) -> Vec<String> {
    [
        ContextKind::SuggestedSubcommand,
        ContextKind::SuggestedArg,
        ContextKind::SuggestedValue,
    ]
    .into_iter()
    .filter(|kind| error.get(*kind).is_some())
    .map(|kind| format!("tip: did you mean '{}'?", defined(error, kind, "' or '")))
    .collect()
}

fn without_value(error: &clap::Error) -> UsageError {
    let arg = error
        .get(ContextKind::InvalidArg)
        .map(ToString::to_string)
        .unwrap_or_default();
    if matches!(error.get(ContextKind::InvalidValue), Some(ContextValue::String(value)) if value.is_empty())
    {
        return UsageError::new(format!("a value is required for '{arg}'"));
    }
    let reason = std::error::Error::source(error)
        .map(|reason| format!(": {reason}"))
        .unwrap_or_default();
    let expected = match error.get(ContextKind::ValidValue) {
        Some(ContextValue::Strings(values)) => format!(": expected one of {}", values.join(", ")),
        _ => String::new(),
    };
    UsageError::new(format!("invalid value for '{arg}'{reason}{expected}"))
}
