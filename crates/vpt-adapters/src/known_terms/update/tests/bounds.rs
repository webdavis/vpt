use super::*;

#[test]
fn a_full_list_allows_duplicates_but_refuses_a_new_term() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("terms");
    let original = "Existing\n".repeat(65_536);
    fs::write(&path, &original).unwrap();
    let duplicate = prepare(&path, "EXISTING").unwrap();
    assert!(!duplicate.added());
    duplicate.publish().unwrap();
    assert_eq!(prepare(&path, "New").unwrap_err(), TermsError::TooLarge);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn output_size_is_bounded_before_publishing() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("terms");
    let original = "x".repeat(65_535) + "\n";
    let original = original.repeat(256);
    assert_eq!(original.len(), 16 * 1024 * 1024);
    fs::write(&path, &original).unwrap();
    assert_eq!(prepare(&path, "New").unwrap_err(), TermsError::TooLarge);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn unicode_term_character_boundary_is_exact() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("terms");
    let text = "語".repeat(65_536);
    prepare(&path, &text).unwrap().publish().unwrap();
    assert_eq!(
        super::super::super::read(&path).unwrap(),
        std::slice::from_ref(&text)
    );
    assert_eq!(
        prepare(&path, &(text + "語")).unwrap_err(),
        TermsError::InvalidText
    );
}
