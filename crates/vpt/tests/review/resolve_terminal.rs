use super::resolution::{identifier, setup};
use super::styled::{piped, same_in_every_format, shown};

#[test]
fn review_resolution_at_a_terminal_marks_the_flag_above_the_open_flags() {
    let (sandbox, id, _) = setup();
    let flag = identifier(&sandbox, &id, "low-confidence");
    let dismiss = ["review", "dismiss", id.as_str(), flag.as_str()];
    let resolved = shown(&sandbox, &dismiss);
    let listed = shown(&sandbox, &["review", "list", id.as_str()]);
    assert_eq!(resolved, format!("  ✓ {flag}: dismissed\n{listed}"));
    assert_eq!(
        shown(&sandbox, &dismiss),
        format!("  ✓ {flag}: already dismissed\n{listed}")
    );
    let plain = piped(&sandbox, &dismiss);
    assert!(plain.starts_with(&format!("Resolved {flag}: dismissed\nReview {id}: ")));
    same_in_every_format(&sandbox, &dismiss);
}
