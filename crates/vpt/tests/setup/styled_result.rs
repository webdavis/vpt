use super::styled::{document, piped, same_in_every_format, shown};
use super::*;

#[test]
fn setup_at_a_terminal_marks_the_written_config_and_its_engine() {
    let sandbox = Sandbox::new("setup-styled");
    let args = ["setup", "--engine", "apple", "--overwrite"];
    let written = shown(&sandbox, &args);
    let config = document(&sandbox, &args)["written"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        written,
        format!("  ✓ {config}: written with main engine apple\n")
    );
    assert_eq!(
        piped(&sandbox, &args),
        format!("wrote {config} (main engine apple)\n")
    );
    same_in_every_format(&sandbox, &args);
}
