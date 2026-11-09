use super::*;
#[test]
fn whole_terms_are_unicode_caseless_possessive_aware_and_never_fuzzy() {
    let mut r = Redactor::new(
        &["Alice", "Straße", "Café", "O'Neil"],
        &["Project Atlas"],
        Options::default(),
    )
    .unwrap();
    let text = r.apply("ALICE’s plan for STRASSE and Cafe\u{301}, O'Neil, Project Atlas; Malice and Alicé stay.").unwrap();
    assert_eq!(
        text,
        "[term 1] plan for [term 2] and [term 3], [term 4], [note 5]; Malice and Alicé stay."
    );
    assert_eq!(r.mappings()[0].value, "ALICE’s");
    assert!(!format!("{:?}", r.mappings()).contains("ALICE"));
    r.verify(&text).unwrap();
}
#[test]
fn longest_match_wins_then_class_precedence_and_numbering_follows_source_order() {
    let mut r = Redactor::new(
        &["Alice", "team@example.org", "New York"],
        &["team@example.org", "York City"],
        Options::default(),
    )
    .unwrap();
    assert_eq!(
        r.apply("Alice@example.org team@example.org New York City $1234.50; 1234")
            .unwrap(),
        "[email 1] [term 2] New [note 3] [money 4]; [number 5]"
    );
}
#[test]
fn configured_classes_are_optional_but_terms_and_names_always_apply() {
    let options = Options {
        classes: vec![],
        mask: "<{class}:{n}>".into(),
    };
    let mut r = Redactor::new(&["Alice"], &["Atlas"], options).unwrap();
    assert_eq!(
        r.apply("Alice Atlas a@b www.example.org 1234").unwrap(),
        "<term:1> <note:2> a@b www.example.org 1234"
    );
}
#[test]
fn pattern_classes_handle_unicode_digits_currency_and_bounded_phone_runs() {
    let mut r = Redactor::new(&[], &[], Options::default()).unwrap();
    assert_eq!(
        r.apply(
            "a@b https://example.org/a www.example.org +1 (303) 555-0199; 123; ₹१२३४; １２３４; 99€"
        )
        .unwrap(),
        "[email 1] [url 2] [url 3] [phone 4]; 123; [money 5]; [number 6]; [money 7]"
    );
}
#[test]
fn numbering_continues_across_blocks_and_restarts_for_a_new_copy() {
    let mut r = Redactor::new(&["Alice"], &[], Options::default()).unwrap();
    assert_eq!(r.apply("Alice").unwrap(), "[term 1]");
    assert_eq!(r.apply("Alice").unwrap(), "[term 2]");
    let mut next = Redactor::new(&["Alice"], &[], Options::default()).unwrap();
    assert_eq!(next.apply("Alice").unwrap(), "[term 1]");
}
#[test]
fn residue_refuses_removed_values_terms_and_names_without_exposing_them() {
    let mut r = Redactor::new(&["Alice"], &["Atlas"], Options::default()).unwrap();
    r.apply("private@example.org").unwrap();
    for text in ["ALICE", "Atlas", "private@example.org"] {
        let error = r.verify(text).unwrap_err();
        assert!(matches!(error, Error::Residue { .. }));
        assert!(!format!("{error:?}").contains(text));
    }
    r.verify("Malice Atlantia").unwrap();
    let mut collision = Redactor::new(
        &["Alice"],
        &[],
        Options {
            mask: "Alice {n}".into(),
            ..Options::default()
        },
    )
    .unwrap();
    let text = collision.apply("Alice").unwrap();
    assert!(matches!(
        collision.verify(&text),
        Err(Error::Residue { .. })
    ));
}
#[test]
fn invalid_templates_and_excessive_input_refuse_before_unbounded_work() {
    for mask in ["missing number", "{n}{n}", "{unknown}{n}", "\0{n}"] {
        assert!(matches!(
            Redactor::new(
                &[],
                &[],
                Options {
                    mask: mask.into(),
                    ..Options::default()
                }
            ),
            Err(Error::InvalidMask)
        ));
    }
    let mut r = Redactor::new(&[], &[], Options::default()).unwrap();
    assert_eq!(
        r.apply(&"x".repeat(16 * 1024 * 1024 + 1)),
        Err(Error::Capacity)
    );
    assert_eq!(r.apply("private\0value"), Err(Error::InvalidText));
}

#[test]
fn long_numeric_amounts_are_scanned_once_and_failed_capacity_does_not_advance_numbering() {
    let options = Options {
        classes: vec![Class::Money, Class::Number],
        ..Default::default()
    };
    let mut r = Redactor::new(&[], &[], options).unwrap();
    let source = format!("{}1€", "1.".repeat(10_000));
    assert_eq!(r.apply(&source).unwrap(), "[money 1]");
    assert_eq!(r.apply(&"1234 ".repeat(65_537)), Err(Error::Capacity));
    assert_eq!(r.apply("1234").unwrap(), "[number 2]");
}

#[test]
fn generated_uncertainty_markers_cannot_be_consumed_by_a_vocabulary_match() {
    let marker = 6..18;
    let mut r = Redactor::new(&["Alice [unverified]"], &[], Options::default()).unwrap();
    assert!(matches!(
        r.apply_protected("Alice [unverified]", std::slice::from_ref(&marker)),
        Err(Error::Residue { .. })
    ));
    assert!(r.mappings().is_empty());
    let mut r = Redactor::new(&["Alice"], &[], Options::default()).unwrap();
    assert_eq!(
        r.apply_protected("Alice [unverified]", std::slice::from_ref(&marker))
            .unwrap(),
        "[term 1] [unverified]"
    );
}

#[test]
fn residue_of_a_removed_term_or_name_identifies_its_original_mask_number() {
    let mut r = Redactor::new(&["Alice"], &["Atlas"], Options::default()).unwrap();
    r.apply("Alice Atlas a@b").unwrap();
    for (text, class, number) in [
        ("ALICE", Class::Term, 1),
        ("ATLAS", Class::Note, 2),
        ("a@b", Class::Email, 3),
    ] {
        assert_eq!(
            r.verify(text),
            Err(Error::Residue {
                class,
                number: Some(number)
            })
        );
    }
}
