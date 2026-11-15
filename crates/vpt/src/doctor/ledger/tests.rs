use super::*;

fn assert_error(error: LedgerError, expected: &str) {
    let row = source_gone(Err(error));
    assert_eq!(
        (row.name.as_str(), row.verdict, row.detail.as_str()),
        ("source_gone", crate::style::Verdict::Issue, expected)
    );
    assert!(!row.detail.contains("private-canary"));
}

#[test]
fn source_gone_busy_is_safe() {
    assert_error(LedgerError::Busy, "ledger is busy");
}
#[test]
fn source_gone_schema_is_safe() {
    assert_error(
        LedgerError::UnsupportedSchema(4321),
        "unsupported ledger schema",
    );
}
#[test]
fn source_gone_io_is_safe() {
    assert_error(
        LedgerError::Io("private-canary".into()),
        "ledger operation failed",
    );
}
#[test]
fn source_gone_conflict_is_safe() {
    assert_error(
        LedgerError::Conflict("private-canary".into()),
        "ledger conflict",
    );
}
#[test]
fn source_gone_corrupt_is_safe() {
    assert_error(
        LedgerError::Corrupt("private-canary".into()),
        "ledger is corrupt",
    );
}
#[test]
fn source_gone_escape_is_safe() {
    assert_error(
        LedgerError::PathEscape("/owned/private-canary".into()),
        "ledger path escapes its retained state root",
    );
}
