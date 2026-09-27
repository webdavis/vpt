use std::io::Write;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) enum Outcome {
    Success {
        document: serde_json::Value,
        human: String,
    },
    Failure(ErrorDocument),
    ParseFailure(ErrorDocument),
    FailedReport {
        error: ErrorDocument,
        human: String,
    },
}

pub(crate) fn emit(outcome: Outcome, json: bool) -> i32 {
    emit_to(
        outcome,
        json,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
}

fn emit_to(outcome: Outcome, json: bool, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parser_help = matches!(&outcome, Outcome::ParseFailure(_));
    match outcome {
        Outcome::FailedReport { error, human } => {
            let text = if json {
                failure_text(&error, true, false)
            } else {
                format!("{human}{}", failure_text(&error, false, false))
            };
            write_failure(&error, &text, stderr)
        }
        Outcome::Success { document, human } => {
            let text = if json { format!("{document}\n") } else { human };
            if stdout.write_all(text.as_bytes()).is_err() {
                return emit_to(
                    Outcome::Failure(ErrorDocument::new(
                        ErrorKind::Store,
                        "standard output write failed",
                    )),
                    json,
                    stdout,
                    stderr,
                );
            }
            0
        }
        Outcome::Failure(error) | Outcome::ParseFailure(error) => {
            let text = failure_text(&error, json, parser_help);
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
    mod reports;
    use super::*;
    use vpt_application::ports::LedgerError;
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

    fn assert_safe_ledger_fault(fault: LedgerError, message: &str) {
        let error = crate::compose::errors::ledger_error(fault);
        assert_eq!(error.exit_code(), 1);
        let text = failure_text(&error, true, false);
        assert_eq!(text.lines().count(), 1);
        let actual: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            actual,
            serde_json::json!({
                "schema": "vpt.error/1",
                "error": {
                    "kind": "ledger",
                    "rule": null,
                    "message": message,
                    "ids": [],
                    "completed": [],
                }
            })
        );
        assert!(!text.contains("private"));
    }

    #[test]
    fn typed_ledger_fault_busy_emits_safe_complete_error_document() {
        assert_safe_ledger_fault(LedgerError::Busy, "ledger is busy");
    }

    #[test]
    fn typed_ledger_fault_schema_emits_safe_complete_error_document() {
        assert_safe_ledger_fault(
            LedgerError::UnsupportedSchema(789),
            "unsupported ledger schema",
        );
    }

    #[test]
    fn typed_ledger_fault_io_emits_safe_complete_error_document() {
        assert_safe_ledger_fault(
            LedgerError::Io("private io canary".into()),
            "ledger operation failed",
        );
    }

    #[test]
    fn typed_ledger_fault_conflict_emits_safe_complete_error_document() {
        assert_safe_ledger_fault(
            LedgerError::Conflict("private conflict canary".into()),
            "ledger conflict",
        );
    }

    #[test]
    fn typed_ledger_fault_corrupt_emits_safe_complete_error_document() {
        assert_safe_ledger_fault(
            LedgerError::Corrupt("private corrupt canary".into()),
            "ledger is corrupt",
        );
    }
}
