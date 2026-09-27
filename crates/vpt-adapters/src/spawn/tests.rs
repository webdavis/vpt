use super::*;

mod fixture_child;
mod interrupts;
mod lifecycle;
mod lifecycle_descendants;
mod lifecycle_fallback;
mod lifecycle_fixture;
mod native_io;
mod native_io_fixture;
mod peers;
mod post_reap;
mod reaper;
mod watchdog;

#[test]
fn outcome_debug_names_status_and_counts_without_raw_capture() {
    let outcome = Outcome {
        status: Status::Exited(3),
        stdout: b"stdout-private-sentinel".to_vec(),
        stderr: b"stderr-private-sentinel".to_vec(),
        group: 42,
    };
    let diagnostic = format!("{outcome:?}");
    assert!(diagnostic.contains("Exited(3)"), "status missing");
    assert!(
        diagnostic.contains("stdout_bytes: 23"),
        "stdout count missing"
    );
    assert!(
        diagnostic.contains("stderr_bytes: 23"),
        "stderr count missing"
    );
    assert!(
        !diagnostic.contains("private-sentinel"),
        "raw capture leaked"
    );
    assert!(
        !diagnostic.contains("115, 116, 100"),
        "raw byte capture leaked"
    );
}

#[test]
fn empty_argv_is_a_typed_safe_error() {
    assert_eq!(
        run(&[], b"private input", Duration::ZERO, OUTPUT_LIMIT).expect_err("empty argv"),
        SpawnError::Io("empty argv".into())
    );
}

#[test]
fn production_output_and_grace_limits_match_the_process_contract() {
    assert_eq!(OUTPUT_LIMIT, 65_536);
    assert_eq!(GRACE, Duration::from_secs(1));
}

#[test]
fn production_controls_preserve_the_one_second_grace() {
    assert_eq!(production_controls().grace, Duration::from_secs(1));
}
