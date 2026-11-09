use super::*;
use crate::ledger::retention_artifacts::{Inventory, MAX_ARTIFACTS};
use std::path::Path;

fn push(rows: &mut Inventory, path: &str) -> Result<(), LedgerError> {
    rows.push(
        ArtifactKind::Audio,
        contract::record(1).id.as_str(),
        Path::new(path),
        contract::digest(1),
        RetentionStatus::Published,
        None,
    )
}

#[test]
fn duplicate_paths_across_artifact_kinds_are_refused() {
    let mut rows = Inventory::default();
    push(&mut rows, "/audio/one").unwrap();
    rows.push(
        ArtifactKind::EngineOutput,
        contract::record(1).id.as_str(),
        Path::new("/audio/one"),
        contract::digest(2),
        RetentionStatus::Published,
        None,
    )
    .unwrap();
    assert!(rows.finish().is_err());
}

#[test]
fn inventory_rejects_invalid_paths_and_owner_ids_without_private_error_text() {
    for path in [
        "Private relative",
        "/audio/../Private",
        "/audio//Private",
        "/audio/./Private",
        "/audio/Private\0",
    ] {
        let error = push(&mut Inventory::default(), path).unwrap_err();
        assert!(!format!("{error:?}").contains("Private"));
    }
    let error = Inventory::default()
        .push(
            ArtifactKind::Brief,
            "Private malformed identity",
            Path::new("/briefs/one"),
            contract::digest(1),
            RetentionStatus::Published,
            None,
        )
        .unwrap_err();
    assert!(!format!("{error:?}").contains("Private"));
}

#[test]
fn inventory_caps_artifact_count_before_adding_another_entry() {
    let mut rows = Inventory::default();
    for index in 0..MAX_ARTIFACTS {
        push(&mut rows, &format!("/audio/{index}")).unwrap();
    }
    assert!(push(&mut rows, "/audio/overflow").is_err());
    assert_eq!(rows.finish().unwrap().len(), MAX_ARTIFACTS);
}

#[test]
fn inventory_caps_total_metadata_even_below_the_artifact_count_limit() {
    let mut rows = Inventory::default();
    let stem = "a".repeat(60_000);
    let mut accepted = 0;
    for index in 0..300 {
        if push(&mut rows, &format!("/audio/{index}-{stem}")).is_err() {
            break;
        }
        accepted += 1;
    }
    assert!(accepted > 250 && accepted < 280);
    assert_eq!(rows.finish().unwrap().len(), accepted);
}
