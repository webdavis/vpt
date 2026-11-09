use super::*;
use crate::config::load_text;

fn configured(text: &str) -> Result<NoteSettings, ConfigError> {
    let table = load_text(&format!("config_version = 1\n{text}")).unwrap();
    super::super::from_table(&table, Path::new("/synthetic/home")).map(|settings| settings.note)
}

#[test]
fn portable_defaults_keep_markdown_links_and_no_vault_dependency() {
    let settings = configured("").unwrap();
    assert_eq!(settings.profile, NoteProfile::Portable);
    assert_eq!(settings.link_style, LinkStyle::Markdown);
    assert_eq!(settings.slug_max_chars, 60);
    assert_eq!(settings.obsidian.vault_root, None);
    assert_eq!(settings.obsidian.hub_transcripts, "transcripts");
    assert_eq!(settings.obsidian.hub_analysis, "analysis");
    assert_eq!(settings.obsidian.hub_briefs, "briefs");
}

#[test]
fn profile_and_link_style_are_independent_and_preserve_all_overrides() {
    for (profile, expected_profile) in [
        ("portable", NoteProfile::Portable),
        ("obsidian", NoteProfile::Obsidian),
    ] {
        for (style, expected_style) in
            [("markdown", LinkStyle::Markdown), ("wiki", LinkStyle::Wiki)]
        {
            let settings = configured(&format!("[note]\nprofile = '{profile}'\nlink_style = '{style}'\nslug_max_chars = 23\n[note.obsidian]\nvault_root = '~/Vault'\nhub_transcripts = 'Voice notes'\nhub_analysis = '分析'\nhub_briefs = 'Meetings'\n")).unwrap();
            assert_eq!(settings.profile, expected_profile);
            assert_eq!(settings.link_style, expected_style);
            assert_eq!(settings.slug_max_chars, 23);
            assert_eq!(
                settings.obsidian.vault_root,
                Some(PathBuf::from("/synthetic/home/Vault"))
            );
            assert_eq!(settings.obsidian.hub_transcripts, "Voice notes");
            assert_eq!(settings.obsidian.hub_analysis, "分析");
            assert_eq!(settings.obsidian.hub_briefs, "Meetings");
        }
    }
}

#[test]
fn obsidian_requires_an_absolute_nonempty_vault_path_without_echoing_its_value() {
    for path in ["", "private-relative-vault", "~/../Vault"] {
        let result = configured(&format!(
            "[note]\nprofile = 'obsidian'\n[note.obsidian]\nvault_root = '{path}'\n"
        ));
        if path.starts_with("~/") {
            assert!(result.is_ok());
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.key(), Some("note.obsidian.vault_root"));
            assert!(!format!("{error:?}").contains("private-relative-vault"));
        }
    }
}
