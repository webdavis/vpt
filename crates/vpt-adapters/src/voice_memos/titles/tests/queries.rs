use super::fixture::{Fixture, titled};
use super::*;
use std::fs;

pub(super) fn wal(root: &RootDir) {
    let fixture = Fixture::new(
        root,
        &[("20260824 144736-4F3AB19C.m4a", Some("Invoice call"))],
    );
    let wal = fs::read(fixture.container.join("CloudRecordings.db-wal")).unwrap();
    assert!(!wal.is_empty(), "committed fixture row must remain in WAL");
    fixture.unchanged_source(|| {
        let copy = fixture.refresh();
        titled(&copy, "20260824 144736-4F3AB19C.m4a", "Invoice call");
        assert_eq!(copy.title("other.m4a"), TitleLookup::Unavailable);
    });
}

pub(super) fn schema(root: &RootDir) {
    let fixture = Fixture::simple(root);
    fixture
        .source
        .execute_batch("DROP TABLE ZCLOUDRECORDING; CREATE TABLE ZSOMETHING(ZNAME TEXT);")
        .unwrap();
    fixture.unchanged_source(|| {
        assert_eq!(fixture.refresh().title("a.m4a"), TitleLookup::Unavailable)
    });
}

pub(super) fn metacharacters(root: &RootDir) {
    let fixture = Fixture::new(
        root,
        &[
            ("dir/axb.m4a", Some("wrong underscore")),
            ("dir/a_b.m4a", Some("underscore")),
            ("dir/100x.m4a", Some("wrong percent")),
            ("dir/100%.m4a", Some("percent")),
            ("dir/a\\b.m4a", Some("backslash")),
        ],
    );
    let copy = fixture.refresh();
    titled(&copy, "a_b.m4a", "underscore");
    titled(&copy, "100%.m4a", "percent");
    titled(&copy, "a\\b.m4a", "backslash");
    assert_eq!(copy.title("b.m4a"), TitleLookup::Unavailable);
}

pub(super) fn empty(root: &RootDir) {
    let fixture = Fixture::new(
        root,
        &[
            ("empty.m4a", Some("")),
            ("space.m4a", Some(" \t\n")),
            ("null.m4a", None),
            ("kept.m4a", Some("  kept  ")),
        ],
    );
    let copy = fixture.refresh();
    titled(&copy, "kept.m4a", "  kept  ");
    for name in ["empty.m4a", "space.m4a", "null.m4a", "missing.m4a"] {
        assert_eq!(copy.title(name), TitleLookup::Unavailable);
    }
}

pub(super) fn case_sensitive(root: &RootDir) {
    let fixture = Fixture::new(
        root,
        &[
            ("dir/A.m4a", Some("upper")),
            ("dir/a.m4a", Some("lower")),
            ("dir/Only.m4a", Some("only")),
            ("direct.m4a", Some("direct")),
            ("dir/prefixa.m4a", Some("prefix")),
        ],
    );
    let copy = fixture.refresh();
    titled(&copy, "a.m4a", "lower");
    titled(&copy, "A.m4a", "upper");
    titled(&copy, "direct.m4a", "direct");
    assert_eq!(copy.title("only.m4a"), TitleLookup::Unavailable);
    assert_eq!(copy.title("irect.m4a"), TitleLookup::Unavailable);
}

pub(super) fn quoted(root: &RootDir) {
    let fixture = Fixture::table(root, "RENAMED\"TABLE", &[("a.m4a", Some("quoted"))]);
    titled(&fixture.refresh(), "a.m4a", "quoted");
}
