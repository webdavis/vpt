use super::super::*;
use serde_json::json;

struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn success() -> Outcome {
    Outcome::Success {
        document: json!({"schema":"vpt.result/1", "command":"doctor", "checks":[]}),
        human: "ok config\n".into(),
    }
}
fn failure() -> ErrorDocument {
    ErrorDocument::new(ErrorKind::Refused, "1 checks failed")
        .rule("doctor_checks")
        .checks(vec![vpt_protocol::error::Check {
            name: "helper".into(),
            ok: false,
            detail: "absent".into(),
        }])
}
fn report() -> Outcome {
    Outcome::FailedReport {
        error: failure(),
        human: "FAIL helper absent\n".into(),
    }
}

#[test]
fn success_json_uses_only_stdout() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(emit_to(success(), true, &mut out, &mut err), 0);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out).unwrap(),
        json!({"schema":"vpt.result/1","command":"doctor","checks":[]})
    );
    assert!(err.is_empty());
}

#[test]
fn success_human_uses_only_stdout() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(emit_to(success(), false, &mut out, &mut err), 0);
    assert_eq!(out, b"ok config\n");
    assert!(err.is_empty());
}

#[test]
fn failed_report_json_is_one_error_document() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(emit_to(report(), true, &mut out, &mut err), 3);
    assert!(out.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&err).unwrap(),
        failure().to_json()
    );
    assert_eq!(err.iter().filter(|byte| **byte == b'\n').count(), 1);
}

#[test]
fn failed_report_human_lists_checks_before_verdict() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(emit_to(report(), false, &mut out, &mut err), 3);
    assert!(out.is_empty());
    assert_eq!(
        err,
        b"FAIL helper absent\nvpt: doctor_checks: 1 checks failed\n"
    );
}

#[test]
fn stdout_failure_is_safe_in_both_modes() {
    let actual = [false, true].map(|json_mode| {
        let mut err = Vec::new();
        let code = emit_to(success(), json_mode, &mut Broken, &mut err);
        (code, String::from_utf8(err).unwrap())
    });
    let error = ErrorDocument::new(ErrorKind::Store, "standard output write failed");
    assert_eq!(
        actual,
        [
            (1, failure_text(&error, false, false)),
            (1, failure_text(&error, true, false))
        ]
    );
}

#[test]
fn broken_stderr_returns_one_for_every_failure_variant() {
    let mut actual = Vec::new();
    for json_mode in [false, true] {
        for outcome in [
            report(),
            Outcome::Failure(failure()),
            Outcome::ParseFailure(failure()),
        ] {
            let mut out = Vec::new();
            actual.push((emit_to(outcome, json_mode, &mut out, &mut Broken), out));
        }
        actual.push((
            emit_to(success(), json_mode, &mut Broken, &mut Broken),
            Vec::new(),
        ));
    }
    assert_eq!(actual, vec![(1, Vec::<u8>::new()); 8]);
}

#[test]
fn parse_failure_preserves_human_help_and_diagnostics() {
    let error = ErrorDocument::new(ErrorKind::Usage, "bad verb")
        .diagnostics(vec!["safe diagnostic".into()]);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(
        emit_to(Outcome::ParseFailure(error), false, &mut out, &mut err),
        2
    );
    assert_eq!(
        String::from_utf8(err).unwrap(),
        format!(
            "vpt: bad verb\nvpt: safe diagnostic\n{}",
            crate::cli::args::USAGE
        )
    );
    assert!(out.is_empty());
}

#[test]
fn parse_failure_json_contains_no_help_text() {
    let error = ErrorDocument::new(ErrorKind::Usage, "bad verb")
        .diagnostics(vec!["safe diagnostic".into()]);
    let expected = error.to_json();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(
        emit_to(Outcome::ParseFailure(error), true, &mut out, &mut err),
        2
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&err).unwrap(),
        expected
    );
    assert!(out.is_empty());
}
