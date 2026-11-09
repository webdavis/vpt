use super::*;

#[test]
fn move_count_is_exact_for_zero_and_largest_wire_count() {
    assert_eq!(moved_count(0), Ok(0));
    assert_eq!(moved_count(u32::MAX as usize), Ok(u32::MAX));
}

#[test]
fn move_count_refuses_above_wire_range() {
    assert_eq!(
        moved_count(u32::MAX as usize + 1),
        Err(LedgerError::Corrupt(
            "retention move count out of range".into()
        ))
    );
}
