use super::*;
use std::fs;
use vpt_domain::relations::{Kind, Provenance, State, Target};
#[path = "relation_confirm/durability.rs"]
mod durability;

fn confirm(sandbox: &Sandbox, id: &str, relation: &str) -> std::process::Output {
    support::run_with_budget(
        sandbox
            .vpt()
            .args(["confirm", id, "--relation", relation, "--json"]),
        std::time::Duration::from_secs(5),
    )
}

fn external(sandbox: &Sandbox) -> std::path::PathBuf {
    let root = sandbox.path().join("home/.vpt/briefs");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("Private target.md");
    fs::write(&path, "---\naliases: [Alias]\n---\nPrivate external prose").unwrap();
    path
}

fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

#[test]
fn all_four_kinds_confirm_valid_targets_and_preserve_content() {
    let (sandbox, row) = configured();
    let target = external(&sandbox);
    let external_bytes = fs::read(&target).unwrap();
    let previous = fixture::record(&sandbox, 1, StageStates::fresh());
    SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap())
        .unwrap()
        .commit(&LedgerCommit {
            recordings: vec![previous.clone()],
            ..Default::default()
        })
        .unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    fs::write(&first.target, format!("{}\nOperator ending\n", first.text)).unwrap();
    for kind in [
        Kind::Continues,
        Kind::Mentions,
        Kind::Related,
        Kind::Supersedes,
    ] {
        let recording_target = matches!(kind, Kind::Continues | Kind::Supersedes);
        let value = if recording_target {
            previous.id.as_str()
        } else {
            "ALIAS"
        };
        let output = confirm(
            &sandbox,
            row.id.as_str(),
            &format!("{}:{value}", kind.as_str()),
        );
        let result = success(&output);
        assert_eq!(result["confirmed"]["relation"], true);
        assert_eq!(result["confirmed"]["kind"], kind.as_str());
        assert_eq!(result["confirmed"]["changed"], true);
        assert!(!stdout(&output).contains("Private"));
        let rows = ledger(&sandbox).relations(&row.id).unwrap();
        let relation = rows.iter().find(|r| r.kind == kind).unwrap();
        assert_eq!(
            relation.target,
            if recording_target {
                Target::Recording(previous.id.clone())
            } else {
                Target::Note(target.to_str().unwrap().into())
            }
        );
        assert_eq!(relation.state, State::Confirmed);
        assert_eq!(relation.provenance, Provenance::Manual);
        let repeated = success(&confirm(
            &sandbox,
            row.id.as_str(),
            &format!(
                "{}:{}",
                kind.as_str(),
                if recording_target {
                    previous.id.to_string()
                } else {
                    target.display().to_string()
                }
            ),
        ));
        assert_eq!(repeated["confirmed"]["changed"], false);
    }
    let note = saved(&sandbox, &row);
    assert_eq!(note.content_digest, first.content_digest);
    assert!(note.text.ends_with("\nOperator ending\n"));
    assert!(
        note.text
            .contains(&format!("- Supersedes: {} (note unavailable)", previous.id))
    );
    assert_eq!(fs::read(&target).unwrap(), external_bytes);
    let log = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!log.contains("Private target"));
}

#[test]
fn recording_targets_work_before_transcription_and_expired_notes_stay_expired() {
    let sandbox = fixture::configured("relation-without-transcript");
    let first = fixture::record(&sandbox, 1, StageStates::fresh());
    let second = fixture::record(&sandbox, 2, StageStates::fresh());
    fixture::seed(&sandbox, vec![first.clone(), second.clone()]);
    success(&confirm(
        &sandbox,
        second.id.as_str(),
        &format!("supersedes:{}", first.id),
    ));
    let relations = ledger(&sandbox).relations(&second.id).unwrap();
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].target, Target::Recording(first.id));
    assert!(
        ledger(&sandbox)
            .note(&second.id, NoteStage::Transcript)
            .unwrap()
            .is_none()
    );

    let (sandbox, row) = configured();
    external(&sandbox);
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    let sql = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&sql);
    sql.execute("UPDATE recordings SET stage_note='expired'", [])
        .unwrap();
    drop(sql);
    fs::remove_file(&first.target).unwrap();
    success(&confirm(&sandbox, row.id.as_str(), "related:Alias"));
    assert!(!first.target.exists());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Expired
    );
    assert_eq!(ledger(&sandbox).relations(&row.id).unwrap().len(), 1);
}

#[test]
fn invalid_unknown_ambiguous_and_self_targets_are_observational_and_private() {
    let (sandbox, row) = configured();
    let target = external(&sandbox);
    fs::write(
        target.with_file_name("Second.md"),
        "---\naliases: [Alias]\n---\nSecond",
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    for value in [
        "unknown:Private target".to_owned(),
        "related:".into(),
        "related:Private missing".into(),
        "related:Alias".into(),
        format!("supersedes:{}", row.id),
    ] {
        let output = confirm(&sandbox, row.id.as_str(), &value);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn confirming_a_suggestion_preserves_its_provenance_and_other_relations() {
    use vpt_domain::relations::Relation;
    let (sandbox, row) = configured();
    let path = external(&sandbox);
    let proposed = Relation {
        kind: Kind::Related,
        target: Target::Note(path.to_str().unwrap().into()),
        state: State::Suggested,
        provenance: Provenance::Proposal {
            by: "Private agent".into(),
            reason: "Private reason".into(),
            input: row.digest,
        },
        ranges: vec![],
    };
    let other = Relation {
        target: Target::Note(path.with_file_name("Other.md").to_str().unwrap().into()),
        provenance: Provenance::Manual,
        ..proposed.clone()
    };
    relations::update(&sandbox, &row.id, vec![proposed.clone(), other.clone()]);
    success(&confirm(&sandbox, row.id.as_str(), "related:Alias"));
    let rows = ledger(&sandbox).relations(&row.id).unwrap();
    assert!(rows.contains(&Relation {
        state: State::Confirmed,
        ..proposed
    }));
    assert!(rows.contains(&other));
    assert!(
        saved(&sandbox, &row)
            .text
            .contains("proposed by Private agent: Private reason")
    );
}

#[test]
fn obsidian_targets_are_limited_to_the_index_and_self_note_is_refused() {
    let (sandbox, row) = configured();
    let outside = external(&sandbox);
    let vault = sandbox.path().join("vault");
    fs::create_dir_all(&vault).unwrap();
    let external = vault.join("Vault target.md");
    fs::write(&external, "---\naliases: [Alias]\n---\nExternal").unwrap();
    std::os::unix::fs::symlink(&outside, vault.join("Redirected.md")).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[note]\nprofile='obsidian'\nlink_style='wiki'\n[note.obsidian]\nvault_root={vault:?}\n"
        ),
    );
    success(&confirm(&sandbox, row.id.as_str(), "related:Alias"));
    let note = saved(&sandbox, &row);
    assert!(note.text.contains("[[Vault target]]"));
    let before = fixture::snapshot(&sandbox);
    for target in [outside, vault.join("Redirected.md"), note.target] {
        let output = confirm(
            &sandbox,
            row.id.as_str(),
            &format!("related:{}", target.display()),
        );
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn an_owned_target_note_resolves_to_its_recording_for_supersedes() {
    let (sandbox, target) = configured();
    success(&invoke(&sandbox, target.id.as_str()));
    let target_note = saved(&sandbox, &target);
    let source = fixture::record(&sandbox, 8, StageStates::fresh());
    SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap())
        .unwrap()
        .commit(&LedgerCommit {
            recordings: vec![source.clone()],
            ..Default::default()
        })
        .unwrap();
    let name = target_note.target.file_stem().unwrap().to_str().unwrap();
    success(&confirm(
        &sandbox,
        source.id.as_str(),
        &format!("supersedes:{name}"),
    ));
    let links = ledger(&sandbox).relations(&source.id).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, Target::Recording(target.id));
    fs::write(&target_note.target, "Foreign occupant").unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = confirm(&sandbox, source.id.as_str(), &format!("continues:{name}"));
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn explicit_confirmation_preserves_existing_mention_source_ranges() {
    use vpt_domain::relations::{Relation, Rule};
    let (sandbox, row) = configured();
    let path = external(&sandbox);
    let ranges = std::iter::once(0..1).collect::<Vec<_>>();
    relations::update(
        &sandbox,
        &row.id,
        vec![Relation {
            kind: Kind::Mentions,
            target: Target::Note(path.to_str().unwrap().into()),
            state: State::Confirmed,
            provenance: Provenance::Derived(Rule::KnownTerm),
            ranges: ranges.clone(),
        }],
    );
    success(&confirm(&sandbox, row.id.as_str(), "mentions:Alias"));
    let links = ledger(&sandbox).relations(&row.id).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].provenance, Provenance::Manual);
    assert_eq!(links[0].ranges, ranges);
    success(&invoke(&sandbox, row.id.as_str()));
    assert_eq!(ledger(&sandbox).relations(&row.id).unwrap(), links);
}

#[test]
fn a_renamed_self_note_is_not_republished_with_a_self_link() {
    for expired in [false, true] {
        let (sandbox, row) = configured();
        success(&invoke(&sandbox, row.id.as_str()));
        let before = saved(&sandbox, &row);
        if expired {
            let sql = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
            fixture::preserve_wal(&sql);
            sql.execute("UPDATE recordings SET stage_note='expired'", [])
                .unwrap();
        }
        let renamed = before.target.with_file_name("Renamed self.md");
        fs::rename(&before.target, &renamed).unwrap();
        let output = confirm(&sandbox, row.id.as_str(), "related:Renamed self");
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert_eq!(saved(&sandbox, &row), before);
        assert_eq!(fs::read_to_string(renamed).unwrap(), before.text);
        assert!(ledger(&sandbox).relations(&row.id).unwrap().is_empty());
        assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    }
}
