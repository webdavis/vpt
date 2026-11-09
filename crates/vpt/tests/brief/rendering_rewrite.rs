use super::rendering_fixture::{ledger, prepared};
use super::*;
use vpt_adapters::note_document::brief::Document;
use vpt_domain::occasion::OccasionId;

#[test]
fn brief_rewrites_preserve_operator_fields_tags_prose_and_creation_metadata() {
    for profile in ["portable", "obsidian"] {
        for style in ["markdown", "wiki"] {
            let (sandbox, _) = prepared(profile, style);
            let result = json(&invoke(&sandbox, &manual(&[])));
            let id = OccasionId::parse(result["occasion"]["id"].as_str().unwrap()).unwrap();
            let saved = ledger(&sandbox).brief(&id).unwrap().unwrap();
            let mut edited = saved
                .text
                .replace("vptSchema:", "operator: keep\nvptSchema:")
                .replace("tags:\n", "tags:\n  - \"operator-tag\"\n");
            if profile == "obsidian" {
                let start = edited
                    .lines()
                    .find(|line| line.starts_with("startDate:"))
                    .unwrap()
                    .to_owned();
                let description = edited
                    .lines()
                    .find(|line| line.starts_with("description:"))
                    .unwrap()
                    .to_owned();
                edited = edited
                    .replace(&start, "startDate: \"1999-12-31\"")
                    .replace(&description, "description: \"Operator description\"")
                    .replace("status: \"active\"", "status: \"paused\"");
            }
            edited.push_str("\nOperator prose outside the managed block.\n");
            std::fs::write(&saved.target, edited).unwrap();
            json(&invoke(&sandbox, &["brief", id.as_str(), "--json"]));
            let after = ledger(&sandbox).brief(&id).unwrap().unwrap();
            assert_eq!(after.target, saved.target);
            assert_eq!(after.first_written, saved.first_written);
            assert_eq!(after.write_offset, saved.write_offset);
            assert_eq!(after.snapshot.pack, saved.snapshot.pack);
            assert!(after.text.contains("operator: keep"));
            assert!(
                after
                    .text
                    .ends_with("\nOperator prose outside the managed block.\n")
            );
            let document = Document::parse(&after.text).unwrap();
            document.check_owner(&id).unwrap();
            for tag in ["planning", "work", "operator-tag"] {
                assert!(document.tags.contains(&tag.into()));
            }
            assert_eq!(after.text.matches("<!-- vpt:brief start -->").count(), 1);
            assert_eq!(after.text.matches("<!-- vpt:brief end -->").count(), 1);
            if profile == "obsidian" {
                for field in [
                    "startDate: \"1999-12-31\"",
                    "status: \"paused\"",
                    "description: \"Operator description\"",
                ] {
                    assert!(after.text.contains(field));
                }
            }
            assert_eq!(std::fs::read_to_string(&after.target).unwrap(), after.text);
            assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
        }
    }
}

#[test]
fn empty_brief_persists_every_empty_section_and_provenance_field() {
    let sandbox = fixture::configured("brief-empty-acceptance");
    let result = json(&invoke(
        &sandbox,
        &[
            "brief",
            "--title",
            "Empty planning",
            "--at",
            "2026-10-01T10:00:00Z",
            "--json",
        ],
    ));
    let id = OccasionId::parse(result["occasion"]["id"].as_str().unwrap()).unwrap();
    let saved = ledger(&sandbox).brief(&id).unwrap().unwrap();
    assert!(saved.snapshot.pack.sources.is_empty());
    assert!(saved.snapshot.pack.items.is_empty());
    assert!(saved.snapshot.tasks.is_empty());
    assert!(saved.text.contains("vptNotes: []"));
    for section in [
        "Unresolved",
        "From your notes",
        "Open tasks",
        "Not selected",
    ] {
        assert!(
            saved.text.contains(&format!("## {section}\n\nNone.\n")),
            "{section}: {}",
            saved.text
        );
    }
    assert!(saved.text.contains("## Occasion\n"));
    assert!(saved.text.contains("calendar: absent (disabled)"));
    assert!(saved.text.contains("tasks: absent (disabled)"));
}
