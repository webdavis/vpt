use super::*;
use std::{collections::BTreeSet, path::PathBuf};

fn files(root: &Path, found: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, found);
        } else {
            found.insert(path);
        }
    }
}

fn split(name: &str, term: Option<&str>, texts: &[&str], halves: &[(usize, &str)], place: &str) {
    for mark in [false, true] {
        let sandbox = fixture::configured(&format!("{name}-{mark}"));
        let flags = halves
            .iter()
            .filter(|_| mark)
            .map(|(index, text)| recording::flag_word(*index, text))
            .collect();
        let id = recording::seed_segments(&sandbox, flags, texts);
        if let Some(term) = term {
            json(&invoke(&sandbox, &["confirm", "--term", term, "--json"]));
        }
        if mark {
            fixture::write_config(&sandbox, "[share.redact]\nflagged_spans = \"mark\"\n");
        }
        let mut before = BTreeSet::new();
        files(sandbox.path(), &mut before);
        let output = invoke(&sandbox, &["redact", id.as_str(), "--json"]);
        let mut emitted = format!("{}{}", support::stdout(&output), stderr(&output));
        assert_eq!(output.status.code(), Some(3), "{mark} {emitted}");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["rule"], "residue");
        assert_eq!(error["error"]["diagnostics"], serde_json::json!([place]));
        assert!(ledger(&sandbox).releases(&id).unwrap().is_empty());
        let mut after = BTreeSet::new();
        files(sandbox.path(), &mut after);
        for path in after.difference(&before) {
            emitted.push_str(&String::from_utf8_lossy(&fs::read(path).unwrap()));
        }
        let events = sandbox.path().join("state/vpt/events.jsonl");
        emitted.push_str(&fs::read_to_string(events).unwrap_or_default());
        for (_, half) in halves {
            assert!(!emitted.contains(half), "{mark} {half}: {emitted}");
        }
        for store in ["released", "drafts"] {
            let dir = sandbox.path().join("home/.vpt").join(store);
            let mut written = BTreeSet::new();
            if dir.exists() {
                files(&dir, &mut written);
            }
            assert!(written.is_empty(), "{mark} {store}: {written:?}");
        }
    }
}

#[test]
fn a_term_split_across_segments_is_refused_and_writes_nothing() {
    split(
        "redact-split-term",
        Some("Acme Corp"),
        &["Send it to Acme", "Corp today"],
        &[(3, "Acme"), (4, "Corp")],
        "between 00:00 and 00:02",
    );
}

#[test]
fn a_term_masked_whole_once_is_still_refused_where_it_is_split_later() {
    split(
        "redact-split-repeat",
        Some("Acme Corp"),
        &["Acme Corp called", "Send it to Acme", "Corp today"],
        &[(6, "Acme"), (7, "Corp")],
        "between 00:02 and 00:04",
    );
}

#[test]
fn a_phone_number_split_across_segments_is_refused_and_writes_nothing() {
    split(
        "redact-split-phone",
        None,
        &["Please call 303-555", "0123 today"],
        &[(2, "303-555"), (3, "0123")],
        "between 00:00 and 00:02",
    );
}
