use super::*;

#[test]
fn saved_publication_inputs_resume_before_or_after_the_file_was_replaced() {
    for exists in [false, true] {
        for published in [false, true] {
            let temp = crate::test_temp::tempdir().unwrap();
            let path = temp.path().join("terms");
            if exists {
                fs::write(&path, "Before\n").unwrap();
            }
            let update = prepare(&path, "After").unwrap();
            let entry = update.publication(UtcInstant { secs: 30 });
            let bytes = update.content().to_vec();
            if published {
                update.publish().unwrap();
            }
            drop(update);
            let recovered = Update::restore(&entry, bytes.clone()).unwrap();
            recovered.publish().unwrap();
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(recovered.publication(entry.recorded_at), entry);
        }
    }
}

#[test]
fn saved_publication_recovery_preserves_operator_edits() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    fs::write(&path, "Before\n").unwrap();
    let update = prepare(&path, "After").unwrap();
    let entry = update.publication(UtcInstant { secs: 30 });
    let bytes = update.content().to_vec();
    drop(update);
    fs::write(&path, "Operator edit\n").unwrap();
    let recovered = Update::restore(&entry, bytes).unwrap();
    assert_eq!(recovered.publish(), Err(TermsError::Changed));
    assert_eq!(fs::read(&path).unwrap(), b"Operator edit\n");
}

#[test]
fn saved_publication_bytes_must_match_the_intended_digest_and_document_limits() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("missing/terms");
    let update = prepare(&path, "After").unwrap();
    let entry = update.publication(UtcInstant { secs: 30 });
    for bytes in [
        b"Other\n".to_vec(),
        b"private\0".to_vec(),
        vec![b'x'; 16 * 1024 * 1024 + 1],
    ] {
        assert!(Update::restore(&entry, bytes.clone()).is_err());
        let mut valid_hash = entry.clone();
        valid_hash.intended = digest(&bytes);
        if bytes != b"Other\n" {
            assert!(Update::restore(&valid_hash, bytes).is_err());
        }
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}
