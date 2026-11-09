use super::*;
use vpt_application::NoteProfile;

#[test]
fn obsidian_vault_must_exist_and_validation_creates_nothing() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.note.profile = NoteProfile::Obsidian;
    let key = "note.obsidian.vault_root".to_owned();
    for path in [
        None,
        Some(PathBuf::from("relative")),
        Some(temp.path().join("missing")),
    ] {
        settings.note.obsidian.vault_root = path;
        let before = entries(temp.path());
        assert!(resolve(&settings, &temp.path().join("cfg")).is_err());
        assert!(resolve_stores(&settings).is_err());
        assert_eq!(entries(temp.path()), before);
    }
    let file = temp.path().join("a-file");
    fs::write(&file, "private").unwrap();
    settings.note.obsidian.vault_root = Some(file);
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::NotADirectory { key })
    );
}

#[test]
fn shared_vault_permissions_and_contents_are_preserved() {
    let temp = fixture();
    let vault = temp.path().join("vault");
    fs::create_dir(&vault).unwrap();
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(vault.join("personal.md"), "Keep this exactly").unwrap();
    let mut settings = settings_in(temp.path());
    settings.note.profile = NoteProfile::Obsidian;
    settings.note.obsidian.vault_root = Some(vault.clone());
    let roots = resolve(&settings, &temp.path().join("cfg")).unwrap();
    roots.create_leaves().unwrap();
    assert_eq!(
        fs::metadata(&vault).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(entries(&vault), vec!["personal.md"]);
    assert_eq!(
        fs::read_to_string(vault.join("personal.md")).unwrap(),
        "Keep this exactly"
    );
}

#[test]
fn portable_settings_do_not_require_an_unused_vault_to_exist() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.note.obsidian.vault_root = Some(temp.path().join("absent-vault"));
    assert!(resolve(&settings, &temp.path().join("cfg")).is_ok());
    assert!(resolve_stores(&settings).is_ok());
    assert!(!temp.path().join("absent-vault").exists());
}
