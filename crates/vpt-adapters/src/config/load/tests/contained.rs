use super::*;

#[test]
fn a_configuration_leaf_symlink_is_refused() {
    let temp = tempfile::tempdir().expect("temp");
    let target = temp.path().join("target.toml");
    let selected = temp.path().join("config.toml");
    std::fs::write(&target, "config_version = 1\n").expect("target");
    std::os::unix::fs::symlink(&target, &selected).expect("link");
    assert_eq!(
        load_file(&selected),
        Err(ConfigError::PathEscape(selected.display().to_string()))
    );
    assert_eq!(
        std::fs::read_to_string(target).expect("unchanged"),
        "config_version = 1\n"
    );
}

#[test]
fn a_configuration_directory_at_the_leaf_is_refused() {
    let temp = tempfile::tempdir().expect("temp");
    assert_eq!(
        load_file(temp.path()),
        Err(ConfigError::PathEscape(temp.path().display().to_string()))
    );
}
