use super::lifecycle_fixture::Fixture;
use std::os::unix::process::ExitStatusExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::ExitStatus;

#[test]
fn exact_reap_record_accepts_normal_group_termination() {
    let fixture = Fixture::new();
    fixture.record_reaped(ExitStatus::from_raw(libc::SIGKILL));
    assert_eq!(
        fixture
            .recorded_status()
            .expect("exact status recorded")
            .signal(),
        Some(libc::SIGKILL)
    );
    fixture.assert_no_fallback();
}

#[test]
fn fallback_exit_status_is_rejected_without_a_marker() {
    for code in [75, 76] {
        let fixture = Fixture::new();
        fixture.record_reaped(ExitStatus::from_raw(code << 8));
        assert!(!fixture.root.path().join("fallback").exists());
        assert!(
            catch_unwind(AssertUnwindSafe(|| fixture.assert_no_fallback())).is_err(),
            "fallback status cannot pass without its marker"
        );
    }
}

#[test]
fn fallback_marker_is_rejected_even_with_a_signal_status() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.path().join("fallback"), b"expired").expect("owned marker");
    fixture.record_reaped(ExitStatus::from_raw(libc::SIGKILL));
    assert!(
        catch_unwind(AssertUnwindSafe(|| fixture.assert_no_fallback())).is_err(),
        "fallback marker cannot pass with a signal status"
    );
}
