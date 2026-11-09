use super::*;
use vpt_domain::tags::{NewTags, Options};

#[test]
fn tag_defaults_are_typed_and_expand_the_confirmation_path() {
    let settings = settings("").unwrap();
    assert_eq!(settings.tags.options, Options::default());
    assert_eq!(
        settings.tags.known_tags_path,
        Path::new("/u/.config/vpt/known-tags.txt")
    );
}

#[test]
fn all_tag_settings_reach_runtime_without_changing_their_limits() {
    let settings = settings("[tags]\nnew_tags = 'write'\nknown_tags_path = '~/tags.txt'\nmax_per_note = 0\nmax_suggested = 4294967295\n").unwrap();
    assert_eq!(settings.tags.options.new_tags, NewTags::Write);
    assert_eq!(settings.tags.options.max_per_note, 0);
    assert_eq!(settings.tags.options.max_suggested, u32::MAX);
    assert_eq!(settings.tags.known_tags_path, Path::new("/u/tags.txt"));
}

#[test]
fn typed_conversion_refuses_invalid_tag_policy_even_without_schema_validation() {
    let mut table = load_text("config_version = 1").unwrap();
    table
        .get_mut("tags")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("new_tags".into(), Value::String("ask".into()));
    assert_eq!(
        from_table(&table, Path::new("/u")).unwrap_err().key(),
        Some("tags.new_tags")
    );
}
