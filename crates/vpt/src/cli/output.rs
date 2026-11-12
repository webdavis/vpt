use super::format::Format;
use std::io::Write;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) enum Outcome {
    Success {
        document: serde_json::Value,
        human: String,
        completed: Vec<String>,
        diagnostics: Vec<String>,
    },
    Failure(ErrorDocument),
    ParseFailure(ErrorDocument),
    FailedReport {
        error: ErrorDocument,
        human: String,
    },
}

pub(crate) fn emit(outcome: Outcome, format: Option<Format>) -> i32 {
    emit_to(
        outcome,
        format,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
}

fn emit_to(
    outcome: Outcome,
    format: Option<Format>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let parser_help = matches!(&outcome, Outcome::ParseFailure(_));
    match outcome {
        Outcome::FailedReport { error, human } => {
            let text = match format {
                Some(_) => failure_text(&error, format, false),
                None => format!("{human}{}", failure_text(&error, None, false)),
            };
            write_failure(&error, &text, stderr)
        }
        Outcome::Success {
            document,
            human,
            completed,
            diagnostics,
        } => {
            let text = match format {
                Some(format) => format.render(&document),
                None => human,
            };
            if stdout.write_all(text.as_bytes()).is_err() {
                return emit_to(
                    Outcome::Failure(
                        ErrorDocument::new(ErrorKind::Store, "standard output write failed")
                            .completed(completed)
                            .diagnostics(diagnostics),
                    ),
                    format,
                    stdout,
                    stderr,
                );
            }
            0
        }
        Outcome::Failure(error) | Outcome::ParseFailure(error) => {
            let text = failure_text(&error, format, parser_help);
            write_failure(&error, &text, stderr)
        }
    }
}

fn write_failure(error: &ErrorDocument, text: &str, stderr: &mut dyn Write) -> i32 {
    if stderr.write_all(text.as_bytes()).is_err() {
        return 1;
    }
    error.exit_code()
}

fn failure_text(error: &ErrorDocument, format: Option<Format>, parser_help: bool) -> String {
    if let Some(format) = format {
        return format.render(&error.to_json());
    }
    let mut text = match (error.kind, &error.rule) {
        (ErrorKind::Refused, Some(rule)) => format!("vpt: {rule}: {}\n", error.message),
        _ => format!("vpt: {}\n", error.message),
    };
    if !error.completed.is_empty() {
        text.push_str(&format!("Completed: {}\n", error.completed.join(", ")));
    }
    for diagnostic in &error.diagnostics {
        text.push_str(&format!("vpt: {diagnostic}\n"));
    }
    if parser_help {
        text.push_str(super::args::USAGE);
    }
    text
}

impl Outcome {
    pub(crate) fn add_completed(&mut self, ids: impl IntoIterator<Item = String>) {
        let completed = match self {
            Self::Success { completed, .. } => completed,
            Self::Failure(error) | Self::ParseFailure(error) | Self::FailedReport { error, .. } => {
                &mut error.completed
            }
        };
        let mut seen: std::collections::HashSet<_> = completed.iter().cloned().collect();
        for id in ids {
            if seen.insert(id.clone()) {
                completed.push(id);
            }
        }
    }
}

#[cfg(test)]
mod tests;
