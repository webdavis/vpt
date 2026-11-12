use super::*;
use std::fs;

#[test]
fn publishing_an_already_selected_tag_preserves_its_spelling() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known-tags.txt");
    prepare_confirmed(&path, "Project_Finance")
        .unwrap()
        .publish()
        .unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "Project_Finance\n");
    let duplicate = prepare_confirmed(&path, "project-finance").unwrap();
    assert!(!duplicate.added());
    for invalid in ["two words", "#tag", "123"] {
        assert_eq!(
            prepare_confirmed(&path, invalid).unwrap_err(),
            TermsError::InvalidText
        );
    }
}

#[test]
fn separator_and_case_variants_preserve_existing_spelling_and_file_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known-tags.txt");
    let original = " Project_Finance \r\nStraße\n";
    fs::write(&path, original).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    for tag in ["project-finance", "PROJECT/FINANCE", "STRASSE"] {
        let update = prepare(&path, tag).unwrap();
        assert!(!update.added());
        update.publish().unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    }
    assert_eq!(read(&path).unwrap(), ["Project_Finance", "Straße"]);
}

#[test]
fn multiword_tags_are_normalized_without_changing_existing_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known-tags.txt");
    fs::write(&path, "Existing_Tag").unwrap();
    let update = prepare(&path, "  Private  Topic ").unwrap();
    assert_eq!(update.content(), b"Existing_Tag\nprivate-topic\n");
    assert_eq!(fs::read_to_string(&path).unwrap(), "Existing_Tag");
    update.publish().unwrap();
    assert_eq!(read(&path).unwrap(), ["Existing_Tag", "private-topic"]);
}

#[test]
fn invalid_proposals_and_invalid_saved_vocabulary_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known-tags.txt");
    for tag in ["#private", "1234", "private!", "", "\0private"] {
        assert_eq!(prepare(&path, tag).unwrap_err(), TermsError::InvalidText);
        assert!(!path.exists());
    }
    for text in ["#private\n", "two words\n", "123\n", "private!\n"] {
        fs::write(&path, text).unwrap();
        assert_eq!(read(&path), Err(TermsError::InvalidText));
        assert_eq!(
            prepare(&path, "valid").unwrap_err(),
            TermsError::InvalidText
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
}

#[test]
fn preparing_missing_tags_is_observational_and_preserves_later_edits() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("missing/known-tags.txt");
    assert!(read(&path).unwrap().is_empty());
    let update = prepare(&path, "Private topic").unwrap();
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    fs::create_dir(path.parent().unwrap()).unwrap();
    fs::write(&path, "operator\n").unwrap();
    assert_eq!(update.publish(), Err(TermsError::Changed));
    assert_eq!(fs::read_to_string(&path).unwrap(), "operator\n");
}

#[test]
fn renaming_replaces_every_spelling_of_the_old_tag_once_and_keeps_other_lines() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known-tags.txt");
    let rename = vpt_domain::tags::Vocabulary::new(&[])
        .unwrap()
        .rename("old topic", "Fresh_Name")
        .unwrap();
    fs::write(&path, " Keep \n old_topic \r\nOther\nOLD-TOPIC").unwrap();
    let update = prepare_renamed(&path, &rename).unwrap();
    assert_eq!(update.content(), b" Keep \nfresh_name\nOther\n");
    update.publish().unwrap();
    assert_eq!(read(&path).unwrap(), ["Keep", "fresh_name", "Other"]);
    assert!(!prepare_renamed(&path, &rename).unwrap().added());
    fs::write(&path, "old-topic\nfresh-name\n").unwrap();
    assert_eq!(
        prepare_renamed(&path, &rename).unwrap().content(),
        b"fresh-name\n"
    );
}
