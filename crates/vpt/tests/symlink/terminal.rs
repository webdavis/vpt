use super::styled::{document, piped, same_in_every_format, shown};
use super::*;

#[test]
fn symlink_at_a_terminal_marks_the_link_and_where_it_points() {
    let (sandbox, _) = configured();
    let deploy = ["symlink", "deploy"];
    let verify = ["symlink", "verify"];
    let deployed = shown(&sandbox, &deploy);
    let result = document(&sandbox, &verify);
    let (link, target) = (&result["link"], &result["target"]);
    let (link, target) = (link.as_str().unwrap(), target.as_str().unwrap());
    assert_eq!(deployed, format!("  ✓ {link}: links to {target}\n"));
    assert_eq!(
        shown(&sandbox, &verify),
        format!("  ✓ {link}: points at {target}\n")
    );
    assert_eq!(piped(&sandbox, &verify), format!("{link} -> {target}\n"));
    same_in_every_format(&sandbox, &verify);
}
