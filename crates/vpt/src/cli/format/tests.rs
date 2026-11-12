use super::Format;
use serde_json::{Value, json};

fn awkward() -> Value {
    json!({
        "schema": "vpt.result/1",
        "texts": ["", "true", "null", "12", "-x", "-", "a,b", "a: b", "say \"hi\"", "back\\slash",
                  "line\nbreak", "tab\there", " padded ", "#hash", "[x]", "{y}", "é ü", "yes", "~", "bell\u{7}"],
        "numbers": [0, -3, 1.5, 18446744073709551615u64],
        "flags": [true, false, null],
        "empty_object": {},
        "empty_list": [],
        "nested_lists": [[1, 2], [], [[3]], ["a", {"b": 1}]],
        "objects": [{}, {"rows": [{"id": 1}, {"id": 2}], "after": "x"}, {"only": []}],
        "Odd Key": 1, "with:colon": 2, "1": 3, "": 4, "on": 5,
        "deep": {"deeper": {"deepest": [{"leaf": null}]}},
    })
}

fn decoded(format: Format, text: &str) -> Value {
    match format {
        Format::Json => serde_json::from_str(text).map_err(|error| error.to_string()),
        Format::Toon => toon_format::decode_default(text).map_err(|error| error.to_string()),
        Format::Yaml => serde_yaml_ng::from_str(text).map_err(|error| error.to_string()),
    }
    .unwrap_or_else(|error| panic!("{format:?} did not decode: {error}\n{text}"))
}

#[test]
fn each_format_decodes_to_the_document_it_rendered() {
    for document in [awkward(), json!({}), json!({"one": "value"})] {
        for (_, format) in Format::FLAGS {
            let text = format.render(&document);
            assert!(text.ends_with('\n'), "{format:?}: {text:?}");
            assert_eq!(decoded(format, &text), document, "{format:?}\n{text}");
        }
    }
}

#[test]
fn json_is_the_compact_document_on_one_line() {
    assert_eq!(
        Format::Json.render(&json!({"schema": "vpt.result/1", "ids": [1, 2]})),
        "{\"schema\":\"vpt.result/1\",\"ids\":[1,2]}\n"
    );
}

#[test]
fn toon_nests_by_indentation_and_declares_every_array_length() {
    let document = json!({
        "schema": "vpt.result/1",
        "recordings": [{"id": "a", "tags": ["x", "y"]}],
        "stores": {"audio": {"files": 0, "oldest": null}},
        "ids": [],
    });
    assert_eq!(
        Format::Toon.render(&document),
        "schema: vpt.result/1\n\
         recordings[1]:\n  \
           - id: a\n    \
             tags[2]: x,y\n\
         stores:\n  \
           audio:\n    \
             files: 0\n    \
             oldest: null\n\
         ids[0]:\n"
    );
}

#[test]
fn yaml_quotes_every_string_and_writes_list_items_compactly() {
    let document = json!({
        "schema": "vpt.result/1",
        "recordings": [{"id": "a", "tags": ["x"]}],
        "ids": [],
    });
    assert_eq!(
        Format::Yaml.render(&document),
        "schema: \"vpt.result/1\"\n\
         recordings:\n  \
           - id: \"a\"\n    \
             tags:\n      \
               - \"x\"\n\
         ids: []\n"
    );
}
