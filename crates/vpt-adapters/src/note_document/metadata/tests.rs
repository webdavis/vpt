use super::*;

#[test]
fn external_notes_expose_names_and_tags_without_owned_note_requirements() {
    let metadata = read("---\naliases: [Ada, 'A. Lovelace']\ntags:\n  - Project_Finance\nvpt-schema: 99\nvpt-recording: foreign\ncustom: preserve\n---\n# Operator note\n").unwrap();
    assert_eq!(metadata.aliases, ["Ada", "A. Lovelace"]);
    assert_eq!(metadata.tags, ["Project_Finance"]);
}

#[test]
fn single_values_anchors_and_unicode_are_read_without_rewriting() {
    let metadata =
        read("\u{feff}---\r\naliases: &name 'Café'\r\ntags: [finance, *name]\r\n---\r\nBody\r\n")
            .unwrap();
    assert_eq!(metadata.aliases, ["Café"]);
    assert_eq!(metadata.tags, ["finance", "Café"]);
}

#[test]
fn absence_and_empty_frontmatter_provide_no_metadata() {
    for text in ["# Note\n---\nBody", "---\n---\nBody", "---\n \n---\nBody"] {
        let metadata = read(text).unwrap();
        assert!(metadata.aliases.is_empty());
        assert!(metadata.tags.is_empty());
    }
}

#[test]
fn null_properties_are_empty_while_quoted_null_is_literal() {
    for value in ["", "null", "~", "!!null null"] {
        let metadata = read(&format!("---\naliases: {value}\ntags: {value}\n---\n")).unwrap();
        assert!(metadata.aliases.is_empty(), "{value}");
        assert!(metadata.tags.is_empty(), "{value}");
    }
    let metadata = read("---\naliases: 'null'\ntags: ['null']\n---\n").unwrap();
    assert_eq!(metadata.aliases, ["null"]);
    assert_eq!(metadata.tags, ["null"]);
}

#[test]
fn malformed_or_unbounded_metadata_is_refused_without_content_in_debug() {
    for text in [
        "---\naliases: [unfinished\n---\n",
        "---\ntags: {key: value}\n---\n",
        "---\naliases: [one]\naliases: [two]\n---\n",
    ] {
        assert!(read(text).is_err());
    }
    let text = format!("---\naliases: [{}]\n---\n", vec!["alias"; 257].join(","));
    assert!(read(&text).is_err());
    let metadata = read("---\naliases: [PrivateCanary]\ntags: [Secret]\n---\n").unwrap();
    assert!(!format!("{metadata:?}").contains("PrivateCanary"));
    assert!(!format!("{metadata:?}").contains("Secret"));
}
