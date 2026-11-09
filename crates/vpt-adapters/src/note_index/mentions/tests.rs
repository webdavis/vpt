use super::*;
use std::fs;

#[test]
fn mention_resolution_uses_external_names_and_unique_aliases_without_writing_notes() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("Alpha.md");
    let second = temp.path().join("Beta.md");
    let first_text = "---\naliases: [New York, NYC, shared, Alpha]\n---\nPrivate first prose\n";
    let second_text = "---\naliases: [shared]\n---\nPrivate second prose\n";
    fs::write(&first, first_text).unwrap();
    fs::write(&second, second_text).unwrap();
    let index = Index::read(&[temp.path().into()]).unwrap();
    let result = index
        .mentions(
            &["ALPHA", "new", "york", "NYC", "shared", "Alphaish", "Beta"],
            &[
                "Alpha".into(),
                "New York".into(),
                "NYC".into(),
                "shared".into(),
                "missing".into(),
            ],
        )
        .unwrap();
    assert_eq!(result.links.len(), 1);
    assert_eq!(index.notes()[result.links[0].target].path, first);
    assert_eq!(result.links[0].ranges, [0..1, 1..3, 3..4]);
    assert_eq!(result.ambiguous.len(), 1);
    assert_eq!(result.ambiguous[0].term, 3);
    assert_eq!(result.ambiguous[0].ranges.len(), 1);
    assert_eq!(result.ambiguous[0].ranges[0], 4..5);
    assert_eq!(fs::read_to_string(first).unwrap(), first_text);
    assert_eq!(fs::read_to_string(second).unwrap(), second_text);
}
