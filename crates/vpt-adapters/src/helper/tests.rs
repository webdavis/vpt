use super::*;
use crate::spawn::Status;
use serde_json::json;
use std::cell::RefCell;

fn client() -> HelperClient {
    HelperClient::new(PathBuf::from("/sandbox/fake-helper"))
}

fn version() -> Value {
    json!({"schema":"vpt.helper/1", "version":"1.2.3-rc.1+build.7"})
}

fn outcome(status: Status, stdout: Vec<u8>) -> spawn::Outcome {
    spawn::Outcome {
        status,
        stdout,
        stderr: b"CANARY-STDERR".to_vec(),
        group: 0,
    }
}

#[test]
fn compatible_version_is_cached_before_trash_and_reused() {
    let client = client();
    let calls = RefCell::new(Vec::new());
    let call = |argv: &[String]| {
        calls.borrow_mut().push(argv[0].clone());
        Ok(if argv[0] == "--version" {
            version()
        } else {
            json!({"trashed":argv[1]})
        })
    };
    assert_eq!(client.version_using(&call), Ok("1.2.3-rc.1+build.7".into()));
    let path = Path::new("/sandbox/file");
    assert_eq!(client.trash_using(path, &call), Ok(path.into()));
    assert_eq!(client.version_using(&call), Ok("1.2.3-rc.1+build.7".into()));
    assert_eq!(*calls.borrow(), ["--version", "trash"]);
}

#[test]
fn incompatible_version_is_rechecked_and_trash_is_never_called() {
    let client = client();
    let calls = RefCell::new(Vec::new());
    let call = |argv: &[String]| {
        calls.borrow_mut().push(argv[0].clone());
        Ok(json!({"schema":"vpt.helper/1", "version":"2.3.0"}))
    };
    assert_eq!(
        client.version_using(&call),
        Err(HelperError::MajorMismatch { found: 2 })
    );
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &call),
        Err(TrashError::HelperVersion { found: 2 })
    );
    assert_eq!(*calls.borrow(), ["--version", "--version"]);
}

#[test]
fn failed_version_is_not_cached() {
    let client = client();
    let calls = RefCell::new(0);
    let call = |_: &[String]| {
        *calls.borrow_mut() += 1;
        Err(HelperError::Absent)
    };
    assert_eq!(client.version_using(&call), Err(HelperError::Absent));
    assert_eq!(client.version_using(&call), Err(HelperError::Absent));
    assert_eq!(*calls.borrow(), 2);
}

#[test]
fn trash_reply_must_name_the_exact_requested_path() {
    let client = client();
    let call = |argv: &[String]| {
        Ok(if argv[0] == "--version" {
            version()
        } else {
            json!({"trashed":"/sandbox/another"})
        })
    };
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &call),
        Err(TrashError::Unknown(
            "the helper trash reply names another path".into()
        ))
    );
    let noncanonical = |argv: &[String]| {
        Ok(if argv[0] == "--version" {
            version()
        } else {
            json!({"trashed":"/sandbox//file"})
        })
    };
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &noncanonical),
        Err(TrashError::Unknown(
            "the helper trash reply names another path".into()
        ))
    );
}

#[test]
fn invalid_reply_types_and_schemas_hide_child_values() {
    let client = client();
    for schema in [json!("CANARY-SCHEMA"), json!({"private":"CANARY-SCHEMA"})] {
        assert_eq!(
            client.version_using(&|_| Ok(json!({"schema":schema, "version":"1.0.0"}))),
            Err(HelperError::Unknown("invalid helper schema".into()))
        );
    }
    assert_eq!(
        client.version_using(&|_| Ok(json!({"schema":"vpt.helper/9", "version":"1.0.0"}))),
        Err(HelperError::Unknown(
            "unsupported helper schema major 9".into()
        ))
    );
    for value in [json!("1.0.0+CANARY\nSECRET"), json!({"private":"CANARY"})] {
        assert_eq!(
            client.version_using(&|_| Ok(json!({"schema":"vpt.helper/1", "version":value}))),
            Err(HelperError::Unknown(
                "invalid helper version document".into()
            ))
        );
    }
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &|argv| Ok(
            if argv[0] == "--version" {
                version()
            } else {
                json!({"trashed":["CANARY-TRASH"]})
            }
        )),
        Err(TrashError::Unknown("invalid helper trash document".into()))
    );
}

#[test]
fn additive_field_names_are_escaped_bounded_and_drained_once() {
    let client = client();
    let call = |argv: &[String]| {
        let mut value = if argv[0] == "--version" {
            version()
        } else {
            json!({"trashed":argv[1]})
        };
        value["bad\nkey"] = json!("CANARY-PRIVATE-VALUE");
        for index in 0..30 {
            let key = format!("future{index:02}");
            value[key.as_str()] = json!("CANARY-PRIVATE-VALUE");
        }
        Ok(value)
    };
    client
        .trash_using(Path::new("/sandbox/file"), &call)
        .expect("trash reply");
    let diagnostics = client.take_diagnostics();
    assert!(!diagnostics.is_empty());
    assert!(diagnostics.len() <= 16);
    assert!(diagnostics.iter().any(|text| text.contains("bad\\nkey")));
    assert!(
        diagnostics
            .iter()
            .all(|text| text.len() <= 400 && !text.contains("CANARY"))
    );
    assert!(client.take_diagnostics().is_empty());
}

#[test]
fn extra_stdout_byte_rejects_an_oversized_valid_prefix() {
    let client = client();
    let error = client.call_using(
        &["--version".into()],
        &[],
        |argv, stdin, deadline, limit, _| {
            assert_eq!(argv, &["/sandbox/fake-helper", "--version"]);
            assert!(stdin.is_empty());
            assert_eq!(deadline, CALL_DEADLINE);
            assert_eq!(limit, 65_537);
            let mut stdout = serde_json::to_vec(&version()).expect("json");
            stdout.resize(limit, b' ');
            Ok(outcome(Status::Exited(0), stdout))
        },
    );
    assert_eq!(
        error,
        Err(HelperError::Unknown(
            "invalid or oversized helper document".into()
        ))
    );
}

#[test]
fn runner_failures_and_statuses_are_typed_without_child_output() {
    let client = client();
    let call = |result| client.call_using(&[], &[], |_, _, _, _, _| result);
    assert_eq!(
        call(Err(SpawnError::NotFound("CANARY".into()))),
        Err(HelperError::Absent)
    );
    assert_eq!(
        call(Err(SpawnError::Io("CANARY".into()))),
        Err(HelperError::Failed("the helper could not start".into()))
    );
    for (status, expected) in [
        (
            Status::Exited(17),
            HelperError::Failed("the helper exited 17".into()),
        ),
        (
            Status::Signaled(9),
            HelperError::Failed("the helper died on signal 9".into()),
        ),
        (
            Status::DeadlineExceeded,
            HelperError::Unknown("the helper exceeded its deadline".into()),
        ),
        (
            Status::Interrupted,
            HelperError::Unknown("interrupted".into()),
        ),
    ] {
        assert_eq!(
            call(Ok(outcome(status, b"CANARY-STDOUT".to_vec()))),
            Err(expected)
        );
    }
}
