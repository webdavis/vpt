use super::super::*;
use vpt_application::{RetentionReport, ports::SinkWarning};
use vpt_domain::identity::RecordingId;

const ID: &str = "2026-08-24T144736-4f3ab19c02de";
const OCCASION: &str = "2026-09-30T230000-cb4b7855ddbc";
const RECOVERED: &str = "2026-08-24T144743-070707070707";

struct PartialOutput;
impl Write for PartialOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > 1 {
            Ok(bytes.len() - 1)
        } else {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn partial_stdout_failure_preserves_commits_recovery_and_safe_warnings() {
    for json in [false, true] {
        let outcome = Outcome::Success {
            document: serde_json::json!({"private": "private content", "id":"uncommitted"}),
            human: "private content\n".into(),
            completed: vec![ID.into()],
            diagnostics: vec!["safe diagnostic".into()],
        };
        let outcome = crate::compose::events::attach(
            Ok(outcome),
            &[SinkWarning::ExportUnavailable].into_iter().collect(),
        );
        let outcome = crate::compose::startup::attach(
            outcome,
            crate::compose::startup::StartupReport {
                publications: vec![OCCASION.into()],
                retention: RetentionReport {
                    completed: [ID, RECOVERED]
                        .map(|id| RecordingId::parse(id).unwrap().into())
                        .to_vec(),
                    ..Default::default()
                },
            },
        );
        let mut stderr = Vec::new();
        assert_eq!(
            emit_to(
                outcome,
                json.then_some(Format::Json),
                &mut PartialOutput,
                &mut stderr
            ),
            1
        );
        let error = String::from_utf8(stderr).unwrap();
        assert!(!error.contains("private"));
        assert!(!error.contains("uncommitted"));
        assert!(error.contains("safe diagnostic"));
        assert!(error.contains("event log export is unavailable"));
        for id in [ID, OCCASION, RECOVERED] {
            assert_eq!(error.matches(id).count(), 1);
        }
        if json {
            let value: serde_json::Value = serde_json::from_str(&error).unwrap();
            assert_eq!(
                value["error"]["completed"],
                serde_json::json!([ID, OCCASION, RECOVERED])
            );
        }
    }
}

#[test]
fn every_human_failure_reports_completed_ids_once_including_recovered_ids() {
    let error = ErrorDocument::new(ErrorKind::Store, "work failed").completed(vec![ID.into()]);
    for outcome in [
        Outcome::Failure(error.clone()),
        Outcome::ParseFailure(error.clone()),
        Outcome::FailedReport {
            error,
            human: "Partial report\n".into(),
        },
    ] {
        let outcome = crate::compose::startup::attach(
            outcome,
            crate::compose::startup::StartupReport {
                publications: vec![OCCASION.into()],
                retention: RetentionReport {
                    completed: [ID, RECOVERED]
                        .map(|id| RecordingId::parse(id).unwrap().into())
                        .to_vec(),
                    ..Default::default()
                },
            },
        );
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        assert_eq!(emit_to(outcome, None, &mut stdout, &mut stderr), 1);
        assert!(stdout.is_empty());
        let error = String::from_utf8(stderr).unwrap();
        for id in [ID, OCCASION, RECOVERED] {
            assert_eq!(error.matches(id).count(), 1);
        }
    }
}
