use super::*;
use crate::contained::private_log::append::AppendStep;

fn log(root: &RootDir) -> EventLog {
    plain(&root.path().join("events.jsonl"), b"{}\nfragment", 0o600);
    root.open_event_log().expect("private log")
}

fn fail_at(step: AppendStep, selected: AppendStep) -> Result<(), LogError> {
    if step == selected {
        Err(LogError::Io(ErrorKind::Other))
    } else {
        Ok(())
    }
}

fn initial(log: &EventLog) -> LogInspection {
    let metadata = log.file.metadata().expect("held identity");
    LogInspection {
        device: metadata.dev(),
        inode: metadata.ino(),
        complete_end_offset: 3,
        physical_end_offset: 11,
    }
}

#[test]
fn repair_sync_failure_retains_unsynced_truncation_evidence() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let inspection = Some(initial(&log));
    let result = log.append_with("{}", |step| fail_at(step, AppendStep::RepairSync));
    assert_eq!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::Io(ErrorKind::Other),
            effects: AppendEffects {
                inspection,
                repair: Some(TailRepair {
                    original_end: 11,
                    retained_end: 3,
                    synced: false
                }),
                ..AppendEffects::default()
            }
        })
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n"
    );
}

#[test]
fn append_failure_retains_synced_repair_evidence() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let inspection = Some(initial(&log));
    let result = log.append_with("{}", |step| fail_at(step, AppendStep::Write));
    assert_eq!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::Io(ErrorKind::Other),
            effects: AppendEffects {
                inspection,
                repair: Some(TailRepair {
                    original_end: 11,
                    retained_end: 3,
                    synced: true
                }),
                ..AppendEffects::default()
            }
        })
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n"
    );
}

#[test]
fn failure_before_final_newline_keeps_partial_write_unacknowledged() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let inspection = Some(initial(&log));
    let mut writes = 0;
    let result = log.append_with("{}", |step| {
        if step == AppendStep::Write {
            writes += 1;
        }
        if writes == 2 {
            Err(LogError::Io(ErrorKind::Other))
        } else {
            Ok(())
        }
    });
    assert_eq!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::Io(ErrorKind::Other),
            effects: AppendEffects {
                inspection,
                repair: Some(TailRepair {
                    original_end: 11,
                    retained_end: 3,
                    synced: true
                }),
                written_bytes: 2,
                synced: AppendSync::None
            }
        })
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n{}"
    );
    let retry = log.append_line("{}").expect("resume partial append");
    assert_eq!(
        retry.repair,
        Some(TailRepair {
            original_end: 5,
            retained_end: 3,
            synced: true
        })
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n{}\n"
    );
}

#[test]
fn file_sync_failure_has_no_confirmed_receipt() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let failure = log
        .append_with("{}", |step| fail_at(step, AppendStep::FileSync))
        .expect_err("file sync failure");
    assert_eq!(failure.error, LogError::Io(ErrorKind::Other));
    assert_eq!(failure.effects.written_bytes, 3);
    assert_eq!(failure.effects.synced, AppendSync::None);
    assert!(failure.effects.repair.expect("retained repair").synced);
}

#[test]
fn directory_sync_failure_retains_only_file_sync_evidence() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let failure = log
        .append_with("{}", |step| fail_at(step, AppendStep::DirectorySync))
        .expect_err("directory sync failure");
    assert_eq!(failure.error, LogError::Io(ErrorKind::Other));
    assert_eq!(failure.effects.written_bytes, 3);
    assert_eq!(failure.effects.synced, AppendSync::File);
}

#[test]
fn replacement_before_confirmation_preserves_effects_without_receipt() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let failure = log
        .append_with("{}", |step| {
            if step == AppendStep::Confirm {
                fs::rename(root.path().join("events.jsonl"), root.path().join("held"))
                    .expect("retain original");
                plain(
                    &root.path().join("events.jsonl"),
                    b"replacement canary",
                    0o600,
                );
            }
            Ok(())
        })
        .expect_err("replacement refusal");
    assert_eq!(failure.error, LogError::Unsafe(LogRefusal::Replaced));
    assert_eq!(failure.effects.written_bytes, 3);
    assert_eq!(failure.effects.synced, AppendSync::FileAndDirectory);
    assert!(failure.effects.repair.expect("repair evidence").synced);
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("replacement bytes"),
        b"replacement canary"
    );
    assert_eq!(
        fs::read(root.path().join("held")).expect("held bytes"),
        b"{}\n{}\n"
    );
}

#[test]
fn changed_length_before_confirmation_has_no_receipt() {
    let (_temp, root) = fixture();
    let mut log = log(&root);
    let failure = log
        .append_with("{}", |step| {
            if step == AppendStep::Confirm {
                use std::io::Write;
                fs::OpenOptions::new()
                    .append(true)
                    .open(root.path().join("events.jsonl"))
                    .expect("owned peer")
                    .write_all(b"peer\n")
                    .expect("peer write");
            }
            Ok(())
        })
        .expect_err("length refusal");
    assert_eq!(failure.error, LogError::LengthChanged);
    assert_eq!(failure.effects.written_bytes, 3);
    assert_eq!(failure.effects.synced, AppendSync::FileAndDirectory);
}
