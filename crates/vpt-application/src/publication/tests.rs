use super::*;
use crate::ports::{DirtyPublication, StoreEntry};
use std::cell::RefCell;
use std::collections::HashMap;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::UtcInstant;

fn digest(bytes: &[u8]) -> Sha256Digest {
    let mut output = [0; 32];
    for (index, byte) in bytes.iter().enumerate() {
        output[index % 32] ^= byte;
    }
    Sha256Digest(output)
}

struct Journal {
    entries: RefCell<Vec<DirtyPublication>>,
    clear_fails: bool,
}

impl PublicationJournal for Journal {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        Ok(self.entries.borrow().clone())
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        if self.clear_fails {
            return Err(LedgerError::Io("clear failed".into()));
        }
        self.entries
            .borrow_mut()
            .retain(|entry| entry.target != target);
        Ok(())
    }
}

struct FakeStores {
    contents: RefCell<HashMap<PathBuf, Vec<u8>>>,
    synced: RefCell<Vec<PathBuf>>,
    sync_fails: bool,
    change_on_sync: bool,
}

impl Stores for FakeStores {
    fn entries(&self, _root: &Path) -> Result<Vec<StoreEntry>, StoreError> {
        Ok(Vec::new())
    }

    fn validate_path(&self, _path: &Path) -> Result<(), StoreError> {
        Ok(())
    }

    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError> {
        Ok(self.contents.borrow().get(path).map(|bytes| digest(bytes)))
    }

    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        digest(bytes)
    }

    fn publish(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        if self.digest_of(target)? != expected {
            return Err(StoreError::Changed(target.to_path_buf()));
        }
        self.contents
            .borrow_mut()
            .insert(target.into(), bytes.into());
        Ok(())
    }

    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError> {
        if self.sync_fails {
            return Err(StoreError::Io("sync failed".into()));
        }
        if self.change_on_sync {
            self.contents
                .borrow_mut()
                .insert(path.to_path_buf(), b"operator".to_vec());
        }
        self.synced.borrow_mut().push(path.into());
        Ok(())
    }
}

fn entry(previous: Option<&[u8]>, intended: &[u8]) -> DirtyPublication {
    DirtyPublication {
        target: PathBuf::from("/synthetic/note.md"),
        expected_previous: previous.map(digest),
        intended: digest(intended),
        recorded_at: UtcInstant { secs: 1 },
    }
}

fn journal(previous: Option<&[u8]>, intended: &[u8], clear_fails: bool) -> Journal {
    Journal {
        entries: RefCell::new(vec![entry(previous, intended)]),
        clear_fails,
    }
}

fn stores(current: Option<&[u8]>, sync_fails: bool) -> FakeStores {
    let contents = current.map_or_else(HashMap::new, |bytes| {
        HashMap::from([(PathBuf::from("/synthetic/note.md"), bytes.into())])
    });
    FakeStores {
        contents: RefCell::new(contents),
        synced: RefCell::new(Vec::new()),
        sync_fails,
        change_on_sync: false,
    }
}

fn no_renderer(_: &Path) -> Option<Vec<u8>> {
    None
}

fn render_new(_: &Path) -> Option<Vec<u8>> {
    Some(b"new".to_vec())
}

#[test]
fn already_intended_target_is_synced_then_completed_without_rendering() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"new"), false);
    let report = repair_publications(&journal, &stores, no_renderer).unwrap();
    assert_eq!(report.completed, vec![PathBuf::from("/synthetic/note.md")]);
    assert!(report.republished.is_empty());
    assert_eq!(*stores.synced.borrow(), report.completed);
    assert!(journal.entries.borrow().is_empty());
}

#[test]
fn already_intended_target_stays_pending_when_directory_sync_fails() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"new"), true);
    assert_eq!(
        repair_publications(&journal, &stores, no_renderer),
        Err(RepairError::Sync {
            path: PathBuf::from("/synthetic/note.md"),
            cause: StoreError::Io("sync failed".into()),
        })
    );
    assert_eq!(journal.entries.borrow().len(), 1);
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"new"
    );
}

#[test]
fn expected_previous_target_is_republished_synced_and_cleared() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"old"), false);
    let report = repair_publications(&journal, &stores, render_new).unwrap();
    assert_eq!(
        report.republished,
        vec![PathBuf::from("/synthetic/note.md")]
    );
    assert!(report.completed.is_empty());
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"new"
    );
    assert_eq!(*stores.synced.borrow(), report.republished);
    assert!(journal.entries.borrow().is_empty());
}

#[test]
fn expected_absent_target_is_published() {
    let journal = journal(None, b"new", false);
    let stores = stores(None, false);
    let report = repair_publications(&journal, &stores, render_new).unwrap();
    assert_eq!(
        report.republished,
        vec![PathBuf::from("/synthetic/note.md")]
    );
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"new"
    );
}

#[test]
fn republished_target_stays_pending_when_directory_sync_fails() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"old"), true);
    assert!(matches!(
        repair_publications(&journal, &stores, render_new),
        Err(RepairError::Sync { .. })
    ));
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"new"
    );
    assert_eq!(journal.entries.borrow().len(), 1);
}

#[test]
fn clear_failure_after_sync_keeps_the_row_pending() {
    for current in [b"old".as_slice(), b"new".as_slice()] {
        let journal = journal(Some(b"old"), b"new", true);
        let stores = stores(Some(current), false);
        assert_eq!(
            repair_publications(&journal, &stores, render_new),
            Err(RepairError::Ledger(LedgerError::Io("clear failed".into())))
        );
        assert_eq!(
            stores.contents.borrow()[Path::new("/synthetic/note.md")],
            b"new"
        );
        assert_eq!(
            *stores.synced.borrow(),
            vec![PathBuf::from("/synthetic/note.md")]
        );
        assert_eq!(journal.entries.borrow().len(), 1);
    }
}

#[test]
fn modified_target_is_refused_without_overwrite_or_clear() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"operator text"), false);
    assert_eq!(
        repair_publications(&journal, &stores, render_new),
        Err(RepairError::TargetModified(PathBuf::from(
            "/synthetic/note.md"
        )))
    );
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"operator text"
    );
    assert!(stores.synced.borrow().is_empty());
    assert_eq!(journal.entries.borrow().len(), 1);
}

#[test]
fn unknown_renderer_leaves_expected_previous_target_pending() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"old"), false);
    assert_eq!(
        repair_publications(&journal, &stores, no_renderer),
        Err(RepairError::Render(PathBuf::from("/synthetic/note.md")))
    );
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"old"
    );
    assert_eq!(journal.entries.borrow().len(), 1);
}

#[test]
fn mismatched_render_digest_leaves_target_pending() {
    let journal = journal(Some(b"old"), b"new", false);
    let stores = stores(Some(b"old"), false);
    let error = repair_publications(&journal, &stores, |_| Some(b"wrong".to_vec()));
    assert_eq!(
        error,
        Err(RepairError::RenderedDigestMismatch(PathBuf::from(
            "/synthetic/note.md"
        )))
    );
    assert_eq!(
        stores.contents.borrow()[Path::new("/synthetic/note.md")],
        b"old"
    );
    assert!(stores.synced.borrow().is_empty());
    assert_eq!(journal.entries.borrow().len(), 1);
}

#[test]
fn changes_during_publication_sync_preserve_the_journal_and_operator_content() {
    for initial in [b"old".as_slice(), b"new".as_slice()] {
        let journal = journal(Some(b"old"), b"new", false);
        let mut stores = stores(Some(initial), false);
        stores.change_on_sync = true;
        assert_eq!(
            repair_publications(&journal, &stores, render_new),
            Err(RepairError::TargetModified(PathBuf::from(
                "/synthetic/note.md"
            )))
        );
        assert_eq!(journal.entries.borrow().len(), 1);
        assert_eq!(
            stores.contents.borrow()[Path::new("/synthetic/note.md")],
            b"operator"
        );
    }
}
