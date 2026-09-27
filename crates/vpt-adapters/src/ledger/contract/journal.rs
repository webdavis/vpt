use super::{digest, publication};
use std::path::PathBuf;
use vpt_application::ports::{LedgerCommit, PublicationJournal, RecordingLedger};

pub(crate) fn pending_publication_lists_until_cleared<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let entry = publication("/h/transcripts/a.md", 4, 1);
    ledger
        .commit(&LedgerCommit {
            publications: vec![entry.clone()],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ledger.pending_publications().unwrap(), vec![entry.clone()]);
    ledger.clear_publication(&entry.target).unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
}

pub(crate) fn same_target_keeps_latest_entry<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let mut entry = publication("/h/transcripts/a.md", 5, 1);
    entry.expected_previous = Some(digest(4));
    ledger
        .commit(&LedgerCommit {
            publications: vec![entry.clone()],
            ..Default::default()
        })
        .unwrap();
    entry.intended = digest(6);
    entry.expected_previous = Some(digest(7));
    entry.recorded_at.secs = 2;
    ledger
        .commit(&LedgerCommit {
            publications: vec![entry.clone()],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ledger.pending_publications().unwrap(), vec![entry]);
}

pub(crate) fn publications_order_by_instant_then_target<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let entries = vec![
        publication("/h/transcripts/z.md", 1, 2),
        publication("/h/transcripts/a/note.md", 4, 1),
        publication("/h/transcripts/b.md", 2, 1),
        publication("/h/transcripts/a.md", 3, 1),
    ];
    ledger
        .commit(&LedgerCommit {
            publications: entries,
            ..Default::default()
        })
        .unwrap();
    let targets: Vec<_> = ledger
        .pending_publications()
        .unwrap()
        .into_iter()
        .map(|entry| entry.target)
        .collect();
    assert_eq!(
        targets,
        ["a.md", "a/note.md", "b.md", "z.md"]
            .map(|name| PathBuf::from(format!("/h/transcripts/{name}")))
    );
}
