use super::*;
use std::fs;
use vpt_application::ports::{NoteLedger, NoteStage, ReviewLedger};
use vpt_domain::uncertainty::Class;

fn paired() -> (Sandbox, vpt_application::ports::RecordingRecord) {
    let (sandbox, row) = uncertainty::apple();
    let parts = [
        "um, we paid 42 to Alice for blue paint and met SecretCarol then SecretCarol on 23rd before heading back home",
        "um, uh erm like, hello. we like this",
    ];
    let segments: Vec<_> = parts
        .iter()
        .enumerate()
        .map(|(part, text)| {
            let words: Vec<_> = text
                .split_whitespace()
                .enumerate()
                .map(|(index, word)| {
                    let start = part as f64 * 6.0 + index as f64 * 0.2;
                    let low = part == 0 && ["um,", "42", "Alice", "blue"].contains(&word);
                    serde_json::json!({"text":word,"start":start,"end":start+0.1,
                "confidence":if low {0.2} else {0.9}})
                })
                .collect();
            serde_json::json!({"start":part as f64*6.0,"end":part as f64*6.0+5.0,
            "text":text,"words":words})
        })
        .collect();
    let document = serde_json::json!({"schema":"vpt.engine/1","engine":{
        "name":"apple","family":"apple","tool":"vpt-macos","version":"1.0.0","model":"SpeechTranscriber"},
        "language":"en-US","local":true,"segments":segments});
    let source = sandbox.path().join("apple.json");
    fs::write(&source, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    fs::write(sandbox.path().join("bin/speech"), format!(
        "#!/bin/sh\nif [ \"$1\" = --version ]; then\nprintf '%s' '{{\"schema\":\"vpt.helper/1\",\"version\":\"1.0.0\"}}'\nelse\n/bin/cat {source:?}\nfi\n"
    )).unwrap();
    let chunks: Vec<_> = document["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|segment| {
            let replace = |text: &str| {
                text.replace("42", "43")
                    .replace("Alice", "Bob")
                    .replace("blue", "green")
                    .replace("23rd", "23")
            };
            let words: Vec<_> = segment["words"]
                .as_array()
                .unwrap()
                .iter()
                .map(|word| {
                    serde_json::json!({"word":replace(word["text"].as_str().unwrap()),
                "start":word["start"],"end":word["end"]})
                })
                .collect();
            serde_json::json!({"timestamp":[segment["start"],segment["end"]],
            "text":replace(segment["text"].as_str().unwrap()),"words":words})
        })
        .collect();
    let backend = sandbox.path().join("whisply.json");
    fs::write(&backend, serde_json::json!({"device":"mlx","model":"large-v3-turbo",
        "transcription":{"en":{"text":chunks.iter().map(|chunk| chunk["text"].as_str().unwrap()).collect::<Vec<_>>().join(" "),"chunks":chunks}}}).to_string()).unwrap();
    let script = sandbox.path().join("checker.sh");
    fs::write(&script, format!("#!/bin/sh\nset -eu\n/bin/mkdir -p \"$5/audio\"\n/bin/cat {backend:?} > \"$5/audio/audio.json\"\n")).unwrap();
    let config = fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("main='apple'", "main='apple'\nchecker='whisply'");
    fs::write(
        sandbox.config_path(),
        format!("{config}\n[engines.whisply]\ncommand=['/bin/sh', {script:?}]\n"),
    )
    .unwrap();
    (sandbox, row)
}

#[test]
fn builtin_pair_classifies_once_by_precedence_and_exposes_ranked_occurrences() {
    let (sandbox, row) = paired();
    let output = run(&sandbox, row.id.as_str(), &[]);
    let result = json(&output);
    let accepted = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    let flags = &accepted.uncertainty.lexical;
    assert!(accepted.uncertainty.diagnostics.is_empty());
    assert_eq!(
        flags.iter().map(|flag| flag.class).collect::<Vec<_>>(),
        [
            Class::Formatting,
            Class::Numeric,
            Class::ProperNoun,
            Class::LowConfidence,
            Class::AgreedUnverified,
            Class::Other,
        ]
    );
    assert_eq!(
        flags
            .iter()
            .map(|flag| flag.record_text.as_str())
            .collect::<Vec<_>>(),
        ["23rd", "42", "Alice", "um,", "SecretCarol", "blue"]
    );
    assert_eq!(flags[4].ranges, [11..12, 13..14]);
    assert_eq!(flags[1].alternative_text.as_deref(), Some("43"));
    assert_eq!(flags[2].alternative_text.as_deref(), Some("Bob"));
    assert_eq!(flags[5].alternative_text.as_deref(), Some("green"));
    assert_eq!(result["flags"]["agreed_unverified"], 2);
    let review = json(&support::run_with_budget(
        sandbox.vpt().args(["review", row.id.as_str(), "--json"]),
        std::time::Duration::from_secs(5),
    ));
    assert_eq!(review["open_flags"], 5);
    assert_eq!(
        review["flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flag| flag["class"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "numeric",
            "proper-noun",
            "low-confidence",
            "agreed-unverified",
            "other"
        ]
    );
    assert_eq!(review["flags"][3]["occurrence_count"], 2);
    assert_eq!(review["flags"][3]["timecode"], "00:02");
    assert_eq!(review["flags"][3]["occurrences"][1]["start"], 2.6);
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(note.text.contains("on 23rd before"));
    assert!(!note.text.contains("[unverified: 23rd"));
    assert_eq!(note.text.matches("[unverified: SecretCarol]").count(), 2);
    for private in ["SecretCarol", "Alice", "green"] {
        assert!(!stdout(&output).contains(private));
        assert!(!stderr(&output).contains(private));
        assert!(
            !fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
                .unwrap()
                .contains(private)
        );
    }
}

#[test]
fn readability_toggle_only_changes_rendered_text_and_preserves_every_lexical_marker() {
    let (sandbox, row) = paired();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let accepted = ledger(&sandbox).accepted_transcript(&row.id).unwrap();
    let review = ledger(&sandbox).review_snapshot(&row.id).unwrap();
    let original = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(original.text.contains("vpt-readability: false"));
    assert!(
        original
            .text
            .contains("[00:06] um, uh erm like, hello. we like this\n")
    );
    let raw: Vec<_> = ["apple", "whisply"]
        .iter()
        .map(|engine| {
            let path = sandbox
                .path()
                .join(format!("home/.vpt/engine-outputs/{}.{engine}.json", row.id));
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    assert_eq!(
        raw[0].1,
        fs::read(sandbox.path().join("apple.json")).unwrap()
    );
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    // Rewriting must use saved state even when both engine fixtures are unavailable.
    fs::remove_file(sandbox.path().join("bin/speech")).unwrap();
    fs::remove_file(sandbox.path().join("checker.sh")).unwrap();
    for enabled in [true, false] {
        fs::write(
            sandbox.config_path(),
            format!("{config}\n[readability]\nenabled={enabled}\n"),
        )
        .unwrap();
        json(&support::run_with_budget(
            sandbox
                .vpt()
                .args(["note", "write", row.id.as_str(), "--json"]),
            std::time::Duration::from_secs(5),
        ));
        let note = ledger(&sandbox)
            .note(&row.id, NoteStage::Transcript)
            .unwrap()
            .unwrap();
        assert!(note.text.contains(&format!("vpt-readability: {enabled}")));
        let expected = if enabled {
            "[00:06] Hello. We like this.\n"
        } else {
            "[00:06] um, uh erm like, hello. we like this\n"
        };
        assert!(note.text.contains(expected), "{}", note.text);
        for marker in [
            "[unverified: um,]",
            "[unverified: 42 | 43]",
            "[unverified: Alice | Bob]",
            "[unverified: blue | green]",
            "[unverified: SecretCarol]",
            "on 23rd before",
        ] {
            assert!(
                original.text.contains(marker),
                "missing original marker: {marker}"
            );
            assert_eq!(
                note.text.matches(marker).count(),
                original.text.matches(marker).count(),
                "{marker}"
            );
        }
        if enabled {
            assert!(note.text.contains("before heading back home.\n"));
            assert_ne!(note.content_digest, original.content_digest);
        } else {
            assert_eq!(note.content_digest, original.content_digest);
        }
        assert_eq!(note.target, original.target);
        assert_eq!(fs::read_to_string(&note.target).unwrap(), note.text);
        assert_eq!(
            ledger(&sandbox).accepted_transcript(&row.id).unwrap(),
            accepted
        );
        assert_eq!(
            ledger(&sandbox)
                .review_snapshot(&row.id)
                .unwrap()
                .unwrap()
                .flags,
            review.as_ref().unwrap().flags
        );
        for (path, bytes) in &raw {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
fn a_single_engine_flags_spelled_ordinals_until_the_whole_term_is_known() {
    let (sandbox, row) = uncertainty::apple();
    let helper = sandbox.path().join("bin/speech");
    fs::write(
        &helper,
        fs::read_to_string(&helper)
            .unwrap()
            .replace("SecretAlice", "thirteenth"),
    )
    .unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let before = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(before.uncertainty.lexical[1].class, Class::AgreedUnverified);
    assert_eq!(before.uncertainty.lexical[1].record_text, "thirteenth");
    assert_eq!(before.uncertainty.lexical[1].ranges, [2..3, 4..5]);
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert_eq!(note.text.matches("[unverified: thirteenth]").count(), 2);
    fs::write(sandbox.path().join("terms.txt"), "THIRTEENTH\n").unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let after = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(after.transcript, before.transcript);
    assert_eq!(after.uncertainty.lexical.len(), 1);
    assert_eq!(after.uncertainty.lexical[0].class, Class::LowConfidence);
}
