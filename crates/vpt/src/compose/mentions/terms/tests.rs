use super::*;
use vpt_adapters::note_index::Index;

#[test]
fn confirming_an_existing_term_at_capacity_keeps_mention_matching_available() {
    let mut terms = vec!["ｓｅｃｒｅｔａｌｉｃｅ".into()];
    terms.extend((1..65_536).map(|n| format!("term{n}")));
    let confirmed = with_learned(&terms, Some("SecretAlice"));
    assert_eq!(confirmed.len(), terms.len());
    assert_eq!(confirmed[0], terms[0]);
    let index = Index::read(&[]).unwrap();
    assert!(index.mentions(&["SecretAlice"], &confirmed).is_ok());
}

#[test]
fn duplicate_confirmation_preserves_first_spelling_and_collapses_whitespace() {
    let terms = vec!["Straße\t  NAME".into()];
    assert_eq!(with_learned(&terms, Some("STRASSE name")).as_ref(), terms);
    assert_eq!(with_learned(&terms, None).as_ref(), terms);
}

#[test]
fn a_new_confirmation_is_available_without_mutating_the_saved_snapshot() {
    let terms = vec!["First".into()];
    assert_eq!(
        with_learned(&terms, Some("Second")).as_ref(),
        ["First", "Second"]
    );
    assert_eq!(terms, ["First"]);
}
