use super::*;

fn refusal(settings: &Settings, temp: &Path) -> Result<Roots, RootError> {
    resolve(settings, &temp.join("cfg"))
}

#[test]
fn vocabulary_lists_behind_a_folder_link_refuse_protected_roots() {
    let temp = fixture();
    let settings = settings_in(temp.path());
    let protected = [
        (settings.state_dir.clone(), "home.state_dir"),
        (temp.path().join("voice-memos"), "source.recordings_dir"),
        (settings.stores.released.clone(), "stores.released"),
    ];
    for (index, (target, second)) in protected.into_iter().enumerate() {
        fs::create_dir_all(&target).expect("protected folder");
        let alias = temp.path().join(format!("alias-{index}"));
        symlink(&target, &alias).expect("folder link");
        for (key, tail) in [
            ("tags.known_tags_path", "tags.txt"),
            ("reconcile.known_terms_path", "missing/terms.txt"),
        ] {
            let mut settings = settings_in(temp.path());
            if key == "tags.known_tags_path" {
                settings.tags.known_tags_path = alias.join(tail);
            } else {
                settings.reconcile.known_terms_path = alias.join(tail);
            }
            assert_eq!(
                refusal(&settings, temp.path()),
                Err(RootError::Overlap {
                    first: key.into(),
                    second: second.into()
                }),
                "{key} behind a link to {second}"
            );
        }
    }
}

#[test]
fn tag_list_behind_a_folder_link_cannot_name_the_term_list() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    let folder = temp.path().join("lists");
    fs::create_dir(&folder).expect("lists");
    symlink(&folder, temp.path().join("alias")).expect("folder link");
    settings.reconcile.known_terms_path = folder.join("terms.txt");
    settings.tags.known_tags_path = temp.path().join("alias/terms.txt");
    assert_eq!(
        refusal(&settings, temp.path()),
        Err(RootError::Overlap {
            first: "tags.known_tags_path".into(),
            second: "reconcile.known_terms_path".into()
        })
    );
}
