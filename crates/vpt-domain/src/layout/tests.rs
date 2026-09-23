use super::*;

fn check_pair(first: RootName, a: &str, second: RootName, b: &str, forbidden: bool) {
    for roots in [
        [(first, Path::new(a)), (second, Path::new(b))],
        [(second, Path::new(b)), (first, Path::new(a))],
    ] {
        let expected = if forbidden {
            Err(RootConflict {
                first: roots[0].0,
                second: roots[1].0,
            })
        } else {
            Ok(())
        };
        assert_eq!(check_overlaps(&roots), expected, "{roots:?}");
    }
}

#[test]
fn home_and_config_cannot_equal_the_source_container() {
    for root in [RootName::Home, RootName::Config] {
        check_pair(root, "/vm", RootName::VoiceMemos, "/vm", true);
    }
}

#[test]
fn home_and_config_cannot_descend_from_the_source_container() {
    for root in [RootName::Home, RootName::Config] {
        check_pair(root, "/vm/new", RootName::VoiceMemos, "/vm", true);
    }
}

#[test]
fn home_and_config_may_be_ancestors_or_unrelated_to_the_source() {
    for root in [RootName::Home, RootName::Config] {
        check_pair(root, "/", RootName::VoiceMemos, "/vm", false);
        check_pair(root, "/vm-other", RootName::VoiceMemos, "/vm", false);
    }
}

#[test]
fn stores_and_state_cannot_overlap_the_source_in_either_direction() {
    for root in [RootName::Store(StoreKey::Audio), RootName::State] {
        for path in ["/vm", "/vm/child", "/"] {
            check_pair(root, path, RootName::VoiceMemos, "/vm", true);
        }
        check_pair(root, "/vm-other", RootName::VoiceMemos, "/vm", false);
    }
}

#[test]
fn stores_are_pairwise_disjoint() {
    for path in ["/a", "/a/child", "/"] {
        check_pair(
            RootName::Store(StoreKey::Audio),
            "/a",
            RootName::Store(StoreKey::Drafts),
            path,
            true,
        );
    }
    check_pair(
        RootName::Store(StoreKey::Audio),
        "/a",
        RootName::Store(StoreKey::Drafts),
        "/ab",
        false,
    );
}

#[test]
fn released_cannot_overlap_private_stores_state_or_config() {
    for root in [
        RootName::Store(StoreKey::Drafts),
        RootName::State,
        RootName::Config,
    ] {
        for path in ["/p", "/p/out", "/"] {
            check_pair(RootName::Store(StoreKey::Released), path, root, "/p", true);
        }
    }
}

#[test]
fn the_home_may_contain_its_stores() {
    assert_eq!(
        check_overlaps(&[
            (RootName::Home, Path::new("/h")),
            (RootName::Store(StoreKey::Audio), Path::new("/h/audio")),
            (
                RootName::Store(StoreKey::Released),
                Path::new("/h/released")
            ),
            (RootName::State, Path::new("/s")),
            (RootName::Config, Path::new("/c")),
            (RootName::VoiceMemos, Path::new("/vm")),
        ]),
        Ok(())
    );
}
