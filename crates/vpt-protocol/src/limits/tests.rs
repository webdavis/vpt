use super::*;

#[test]
fn incoming_limits_are_the_protocol_contract() {
    assert_eq!(
        Limits::incoming(),
        Limits {
            bytes: 65_536,
            depth: 8,
            text_chars: 65_536,
            array_len: 256,
        }
    );
}

#[test]
fn bounded_reader_preserves_json_scalar_kinds() {
    let bytes = br#"[true,false,null,-2,2,1.5,{"nested":[]}]"#;
    assert_eq!(
        read_bounded(bytes, &Limits::incoming()),
        serde_json::from_slice(bytes).map_err(|_| syntax())
    );
}

#[test]
fn byte_limit_covers_whitespace_and_exact_neighbors() {
    let limits = Limits {
        bytes: 2,
        ..Limits::incoming()
    };
    assert_eq!(read_bounded(b"{}", &limits), Ok(serde_json::json!({})));
    assert_eq!(read_bounded(b"{} ", &limits), Err(LimitViolation::Bytes(3)));
    assert_eq!(read_bounded(b"{", &limits), Err(syntax()));
    assert_eq!(
        read_bounded(b"{broken", &limits),
        Err(LimitViolation::Bytes(7))
    );
    let zero = Limits { bytes: 0, ..limits };
    assert_eq!(read_bounded(b"0", &zero), Err(LimitViolation::Bytes(1)));
    assert_eq!(read_bounded(b"", &zero), Err(syntax()));
}

#[test]
fn depth_limit_precedes_malformed_child_and_ignores_string_delimiters() {
    let limits = Limits {
        depth: 1,
        ..Limits::incoming()
    };
    assert!(read_bounded(b"[1]", &limits).is_ok());
    assert!(read_bounded(br#"{"t":"[[[[[["}"#, &limits).is_ok());
    assert_eq!(
        read_bounded(b"[[broken", &limits),
        Err(LimitViolation::Depth(2))
    );
    assert_eq!(
        read_bounded(b"[{}]", &limits),
        Err(LimitViolation::Depth(2))
    );
    let zero = Limits { depth: 0, ..limits };
    assert!(read_bounded(b"null", &zero).is_ok());
    assert_eq!(read_bounded(b"[]", &zero), Err(LimitViolation::Depth(1)));
    assert_eq!(read_bounded(b"{}", &zero), Err(LimitViolation::Depth(1)));
}

#[test]
fn decoded_unicode_text_and_keys_have_exact_character_boundaries() {
    let limits = Limits {
        text_chars: 2,
        ..Limits::incoming()
    };
    assert!(read_bounded(br#"["\u00e9\u00e9"]"#, &limits).is_ok());
    assert!(read_bounded(br#"{"\u00e9\u00e9":0}"#, &limits).is_ok());
    assert_eq!(
        read_bounded(br#"["\u00e9\u00e9\u00e9"]"#, &limits),
        Err(LimitViolation::Text(3))
    );
    assert_eq!(
        read_bounded(br#"{"\u00e9\u00e9\u00e9":null}"#, &limits),
        Err(LimitViolation::Text(3))
    );
    assert_eq!(
        read_bounded(br#"{"abc":broken}"#, &limits),
        Err(LimitViolation::Text(3))
    );
    let zero = Limits {
        text_chars: 0,
        ..limits
    };
    assert!(read_bounded(br#"{"":""}"#, &zero).is_ok());
    assert_eq!(
        read_bounded(br#"["x"]"#, &zero),
        Err(LimitViolation::Text(1))
    );
    assert_eq!(
        read_bounded(br#"{"x":0}"#, &zero),
        Err(LimitViolation::Text(1))
    );
}

#[test]
fn array_limit_precedes_extra_element_decoding() {
    let limits = Limits {
        array_len: 2,
        ..Limits::incoming()
    };
    assert!(read_bounded(b"[1,2]", &limits).is_ok());
    assert_eq!(
        read_bounded(b"[1,2,3]", &limits),
        Err(LimitViolation::Array(3))
    );
    assert_eq!(
        read_bounded(b"[1,2,{broken", &limits),
        Err(LimitViolation::Array(3))
    );
    let zero = Limits {
        array_len: 0,
        ..limits
    };
    assert!(read_bounded(b"[]", &zero).is_ok());
    assert_eq!(
        read_bounded(b"[true]", &zero),
        Err(LimitViolation::Array(1))
    );
    assert_eq!(
        read_bounded(b"[broken", &zero),
        Err(LimitViolation::Array(1))
    );
}

#[test]
fn syntax_and_trailing_documents_have_fixed_safe_errors() {
    for bytes in [b"{".as_slice(), b"{} {}", b"CANARY-RAW-JSON", b"{} CANARY"] {
        assert_eq!(read_bounded(bytes, &Limits::incoming()), Err(syntax()));
    }
    assert!(read_bounded(b"{} \n\t", &Limits::incoming()).is_ok());
}

fn syntax() -> LimitViolation {
    LimitViolation::Syntax("invalid JSON document".into())
}
