use super::styled::{document, piped, same_in_every_format, shown};
use super::*;

#[test]
fn confirm_term_at_a_terminal_says_where_the_term_went() {
    let (sandbox, _) = configured();
    let path = sandbox.config_path().with_file_name("known-terms.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path = {path:?}\n"),
    );
    let args = ["confirm", "term", "Fresh term"];
    let first = shown(&sandbox, &args);
    let path = document(&sandbox, &args)["confirmed"]["path"].clone();
    let path = path.as_str().unwrap();
    assert_eq!(first, format!("  ✓ term: added to {path}\n"));
    assert_eq!(
        shown(&sandbox, &args),
        format!("  ✓ term: already in {path}\n")
    );
    assert_eq!(
        piped(&sandbox, &args),
        format!("Confirmed term in {path}\n")
    );
    same_in_every_format(&sandbox, &args);
}

#[test]
fn confirm_tag_and_reject_tag_at_a_terminal_name_the_recording() {
    let (sandbox, row) = configured();
    let path = sandbox.config_path().with_file_name("known-tags.txt");
    fixture::write_config(&sandbox, &format!("[tags]\nknown_tags_path={path:?}\n"));
    let id = row.id.as_str();
    let confirm = ["confirm", "tag", id, "fresh-topic"];
    assert_eq!(
        shown(&sandbox, &confirm),
        format!("  ✓ {id}: tag confirmed\n  ✓ known tags: added\n")
    );
    assert_eq!(
        shown(&sandbox, &confirm),
        format!("  ✓ {id}: tag already confirmed\n")
    );
    assert_eq!(
        piped(&sandbox, &confirm),
        format!("Confirmed tag for {id}\n")
    );
    same_in_every_format(&sandbox, &confirm);
    assert_eq!(
        shown(&sandbox, &["confirm", "reject-tag", id, "fresh-topic"]),
        format!("  ✓ {id}: tag rejected\n")
    );
}

#[test]
fn confirm_relation_at_a_terminal_names_the_kind() {
    let (sandbox, rows) = configured_with(&[1, 7]);
    let (id, target) = (rows[1].id.as_str(), rows[0].id.as_str());
    success(&invoke(&sandbox, id));
    let relation = format!("continues:{target}");
    let args = ["confirm", "relation", id, relation.as_str()];
    assert_eq!(
        shown(&sandbox, &args),
        format!("  ✓ {id}: continues relation confirmed\n")
    );
    assert_eq!(
        shown(&sandbox, &args),
        format!("  ✓ {id}: continues relation already confirmed\n")
    );
    assert_eq!(
        piped(&sandbox, &args),
        format!("Confirmed relation for {id}\n")
    );
    same_in_every_format(&sandbox, &args);
}
