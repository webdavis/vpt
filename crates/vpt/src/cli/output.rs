use std::io::Write;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) enum Outcome {
    Success {
        document: serde_json::Value,
        human: String,
    },
    Failure(ErrorDocument),
    ParseFailure(ErrorDocument),
}

pub(crate) fn emit(outcome: Outcome, json: bool) -> i32 {
    let parser_help = matches!(&outcome, Outcome::ParseFailure(_));
    match outcome {
        Outcome::Success { document, human } => {
            let text = if json { format!("{document}\n") } else { human };
            if std::io::stdout().write_all(text.as_bytes()).is_err() {
                return emit(
                    Outcome::Failure(ErrorDocument::new(
                        ErrorKind::Store,
                        "standard output write failed",
                    )),
                    json,
                );
            }
            0
        }
        Outcome::Failure(error) | Outcome::ParseFailure(error) => {
            let text = failure_text(&error, json, parser_help);
            if std::io::stderr().write_all(text.as_bytes()).is_err() {
                return 1;
            }
            error.exit_code()
        }
    }
}

fn failure_text(error: &ErrorDocument, json: bool, parser_help: bool) -> String {
    if json {
        return format!("{}\n", error.to_json());
    }
    let mut text = match (error.kind, &error.rule) {
        (ErrorKind::Refused, Some(rule)) => format!("vpt: {rule}: {}\n", error.message),
        _ => format!("vpt: {}\n", error.message),
    };
    for diagnostic in &error.diagnostics {
        text.push_str(&format!("vpt: {diagnostic}\n"));
    }
    if parser_help {
        text.push_str(super::args::USAGE);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use vpt_protocol::error::ErrorKind;

    #[test]
    fn human_failure_includes_each_diagnostic() {
        let error = ErrorDocument::new(ErrorKind::Refused, "engines share a family")
            .rule("same_family")
            .diagnostics(vec!["helper absent".into(), "notification disabled".into()]);
        assert_eq!(
            failure_text(&error, false, false),
            "vpt: same_family: engines share a family\nvpt: helper absent\nvpt: notification disabled\n"
        );
    }

    #[test]
    fn human_usage_error_contains_only_its_message() {
        let error = ErrorDocument::new(ErrorKind::Usage, "unknown id memo");
        let text = failure_text(&error, false, false);
        assert_eq!(text, "vpt: unknown id memo\n");
    }

    #[test]
    fn json_failure_contains_diagnostics_in_one_document() {
        let error = ErrorDocument::new(ErrorKind::Store, "write failed")
            .diagnostics(vec!["helper absent".into()]);
        let text = failure_text(&error, true, false);
        let json: serde_json::Value = serde_json::from_str(&text).expect("one document");
        assert_eq!(json["schema"], "vpt.error/1");
        assert_eq!(
            json["error"]["diagnostics"],
            serde_json::json!(["helper absent"])
        );
        assert!(text.ends_with('\n'));
    }
}
