use super::rendering_fixture::{CONFIRMED, UNCERTAIN, ledger, prepared};
use super::*;
use vpt_adapters::note_document::brief::Document;
use vpt_domain::{brief::Certainty, occasion::OccasionId};

#[test]
fn brief_profiles_keep_exact_evidence_inline_certainty_and_saved_provenance() {
    for profile in ["portable", "obsidian"] {
        for style in ["markdown", "wiki"] {
            let (sandbox, recording) = prepared(profile, style);
            let result = json(&invoke(&sandbox, &manual(&[])));
            let id = OccasionId::parse(result["occasion"]["id"].as_str().unwrap()).unwrap();
            let saved = ledger(&sandbox).brief(&id).unwrap().unwrap();
            let source = ledger(&sandbox)
                .note(&recording, NoteStage::Transcript)
                .unwrap()
                .unwrap();
            let parsed = Document::parse(&saved.text).unwrap();
            parsed.check_owner(&id).unwrap();
            let content = parsed.content().unwrap();
            assert_eq!(saved.snapshot.pack.sources.len(), 1);
            assert_eq!(saved.snapshot.pack.sources[0].recording, recording);
            assert_eq!(saved.snapshot.pack.sources[0].digest, source.digest);
            assert_eq!(saved.snapshot.pack.items.len(), 2);
            assert_eq!(saved.snapshot.pack.items[0].text, CONFIRMED);
            assert_eq!(saved.snapshot.pack.items[0].certainty, Certainty::Confirmed);
            assert_eq!(saved.snapshot.pack.items[1].text, UNCERTAIN);
            assert_eq!(
                saved.snapshot.pack.items[1].certainty,
                Certainty::Unverified
            );
            assert_eq!(saved.snapshot.pack.unresolved.flags, 1);
            assert_eq!(saved.snapshot.pack.unresolved.notes_with_flags, 1);
            assert!(saved.snapshot.pack.unresolved.reviewed);
            for (index, (quote, time, certainty)) in [
                (CONFIRMED, 0.25, "confirmed"),
                (UNCERTAIN, 2.25, "unverified"),
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(result["items"][index]["text"], quote);
                assert_eq!(result["items"][index]["certainty"], certainty);
                assert_eq!(
                    result["items"][index]["sources"][0]["recording"],
                    recording.as_str()
                );
                assert_eq!(result["items"][index]["sources"][0]["at_secs"], time);
                assert_eq!(
                    result["items"][index]["sources"][0]["note"],
                    source.target.to_str().unwrap()
                );
            }
            let stem = source.target.file_stem().unwrap().to_str().unwrap();
            let file = source.target.file_name().unwrap().to_str().unwrap();
            let link = if style == "wiki" {
                format!("[[transcripts/{stem}|{stem}]]")
            } else {
                format!("[{stem}](../transcripts/{file})")
            };
            assert!(
                content.contains(&format!(
                    "- [00:00.25] \"{CONFIRMED}\" — {recording}; {link}; tag\n"
                )),
                "{content}"
            );
            assert!(
                content.contains(&format!(
                    "- [00:02.25] [unverified] \"{UNCERTAIN}\" — {recording}; {link}; tag\n"
                )),
                "{content}"
            );
            assert_eq!(content.matches("[unverified]").count(), 1);
            let mut previous = 0;
            for section in [
                "Occasion",
                "Unresolved",
                "From your notes",
                "Open tasks",
                "Not selected",
            ] {
                let heading = format!("## {section}\n");
                assert_eq!(content.matches(&heading).count(), 1);
                let position = content.find(&heading).unwrap();
                assert!(position >= previous);
                previous = position;
            }
            assert!(content.contains("## Open tasks\n\nNone."));
            assert!(content.contains("vpt confirm --term"));
            for field in [
                "vpt-kind: \"brief\"",
                "vpt-occasion:",
                "vpt-occasion-at:",
                "vpt-source: \"manual\"",
                "vpt-notes:",
                "vpt-unresolved:",
                "vpt-context:",
            ] {
                assert!(saved.text.contains(field), "{field}");
            }
            assert!(saved.text.contains(&source.digest.hex()));
            assert!(saved.text.contains("2026-10-01T10:00:00-06:00"));
            assert!(!saved.text.contains("created-date:"));
            assert!(!saved.text.contains("created-time:"));
            if profile == "obsidian" {
                let date = saved.first_written.civil(saved.write_offset);
                assert!(saved.text.contains(&format!(
                    "start-date: \"{:04}-{:02}-{:02}\"",
                    date.year, date.month, date.day
                )));
                let hub = if style == "wiki" {
                    "[[Planning hub]]"
                } else {
                    "[Planning hub](../Planning%20hub.md)"
                };
                assert!(saved.text.contains(hub), "{}", saved.text);
            }
            assert_eq!(std::fs::read_to_string(&saved.target).unwrap(), saved.text);
            assert_eq!(ledger(&sandbox).brief(&id).unwrap(), Some(saved));
        }
    }
}
