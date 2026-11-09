use super::rendering_fixture::{CONFIRMED, ledger, prepared};
use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn brief_recovery_uses_saved_pack_without_sources_or_context_and_preserves_replacements() {
    for replacement in [false, true] {
        let (sandbox, recording) = prepared("portable", "wiki");
        let database = sandbox.path().join("state/vpt/vpt.db");
        let connection = rusqlite::Connection::open(&database).unwrap();
        fixture::preserve_wal(&connection);
        connection.execute_batch("CREATE TRIGGER refuse_brief_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/briefs/%' BEGIN SELECT RAISE(ABORT,'Private brief interruption'); END").unwrap();
        let output = invoke(&sandbox, &manual(&[]));
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(!stderr(&output).contains("Private"));
        let occasion = ledger(&sandbox).occasions().unwrap().remove(0);
        let saved = ledger(&sandbox).brief(&occasion.id).unwrap().unwrap();
        assert_eq!(saved.snapshot.pack.items[0].text, CONFIRMED);
        assert_eq!(saved.snapshot.pack.sources[0].recording, recording);
        assert!(saved.published_at.is_none());
        assert!(occasion.briefed_at.is_none());
        assert_eq!(ledger(&sandbox).pending_publications().unwrap().len(), 1);
        connection
            .execute_batch("DROP TRIGGER refuse_brief_ack")
            .unwrap();
        drop(connection);
        let note = ledger(&sandbox)
            .note(&recording, NoteStage::Transcript)
            .unwrap()
            .unwrap();
        std::fs::remove_file(&note.target).unwrap();
        std::fs::remove_file(&saved.target).unwrap();
        if replacement {
            std::fs::write(&saved.target, "Operator replacement\n").unwrap();
        }
        let collector = sandbox.path().join("bin/dam");
        std::fs::write(
            &collector,
            format!(
                "#!/bin/sh\n/usr/bin/touch {:?}\nexit 23\n",
                sandbox.path().join("collector-called")
            ),
        )
        .unwrap();
        std::fs::set_permissions(&collector, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = std::fs::read_to_string(sandbox.config_path()).unwrap();
        std::fs::write(sandbox.config_path(), config + "\n[context]\ntype='dam'\ncalendars=['calendar/work/']\nprojects=['work/team/']\n").unwrap();
        let output = invoke(&sandbox, &["note", "write", recording.as_str(), "--json"]);
        assert!(!sandbox.path().join("collector-called").exists());
        let after = ledger(&sandbox).brief(&occasion.id).unwrap().unwrap();
        assert_eq!(after.snapshot, saved.snapshot);
        assert_eq!(after.version, saved.version);
        assert_eq!(after.text, saved.text);
        assert_eq!(after.first_written, saved.first_written);
        if replacement {
            assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
            assert_eq!(
                std::fs::read_to_string(&saved.target).unwrap(),
                "Operator replacement\n"
            );
            assert!(after.published_at.is_none());
            assert!(
                ledger(&sandbox)
                    .occasion(&occasion.id)
                    .unwrap()
                    .unwrap()
                    .briefed_at
                    .is_none()
            );
            assert_eq!(ledger(&sandbox).pending_publications().unwrap().len(), 1);
            assert!(!note.target.exists());
        } else {
            json(&output);
            assert_eq!(std::fs::read_to_string(&saved.target).unwrap(), saved.text);
            assert!(after.published_at.is_some());
            assert!(
                ledger(&sandbox)
                    .occasion(&occasion.id)
                    .unwrap()
                    .unwrap()
                    .briefed_at
                    .is_some()
            );
            assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
        }
    }
}
