use super::*;

fn version(text: &str) -> HelperVersion {
    HelperVersion {
        schema: HELPER_SCHEMA.into(),
        version: text.into(),
    }
}

#[test]
fn helper_version_accepts_semantic_core_prerelease_and_build() {
    for text in [
        "0.0.0",
        "1.2.3",
        "1.2.3-rc.1",
        "1.2.3+build.7",
        "1.2.3-rc.1+build.7",
    ] {
        assert_eq!(
            version(text).major(),
            Some(text.as_bytes()[0] as u32 - u32::from(b'0'))
        );
    }
    assert_eq!(version("4294967295.0.0").major(), Some(u32::MAX));
}

#[test]
fn helper_version_rejects_malformed_core_and_overflow() {
    for text in [
        "",
        "1",
        "1.2",
        "1.2.3.4",
        "v1.2.3",
        "1.x.3",
        "1.2.-3",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "4294967296.0.0",
    ] {
        assert_eq!(version(text).major(), None, "{text}");
    }
}

#[test]
fn helper_version_rejects_malformed_prerelease_and_build() {
    for text in [
        "1.2.3-",
        "1.2.3+",
        "1.2.3-.rc",
        "1.2.3-rc.",
        "1.2.3+build.",
        "1.2.3-01",
        "1.2.3-rc.01",
        "1.2.3+bad space",
        "1.2.3-α",
        "1.2.3+α",
        "1.2.3-rc+build+again",
    ] {
        assert_eq!(version(text).major(), None, "{text}");
    }
    assert_eq!(version("1.2.3+build.01").major(), Some(1));
}

#[test]
fn reply_types_deserialize_required_fields_and_allow_additive_fields() {
    let reply: HelperVersion =
        serde_json::from_str(r#"{"schema":"vpt.helper/1","version":"1.2.3","future":true}"#)
            .expect("version reply");
    assert_eq!(reply.major(), Some(1));
    let trashed: Trashed =
        serde_json::from_str(r#"{"trashed":"/sandbox/file","future":true}"#).expect("trash reply");
    assert_eq!(trashed.trashed, "/sandbox/file");
    assert!(serde_json::from_str::<HelperVersion>(r#"{"schema":"vpt.helper/1"}"#).is_err());
    assert!(serde_json::from_str::<Trashed>(r#"{"trashed":true}"#).is_err());
}
