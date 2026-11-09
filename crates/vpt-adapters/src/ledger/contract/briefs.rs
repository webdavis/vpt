mod inputs;
pub(crate) use inputs::*;
pub(crate) mod fixture;
use super::occasions::saved;
use fixture::*;
use vpt_application::ports::*;
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(crate) fn accepted_brief_pins_its_path_and_keeps_the_occasion_unbriefed(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal),
) {
    let (run, row) = occasion(ledger);
    let artifact = artifact(&row, 10);
    let receipt = saved(save(ledger, run, &row, None, &artifact));
    assert!(receipt.completed.is_empty());
    assert_eq!(ledger.brief(&row.id).unwrap(), Some(artifact.clone()));
    assert_eq!(
        ledger.brief_at(&artifact.target).unwrap(),
        Some(artifact.clone())
    );
    let row = ledger.occasion(&row.id).unwrap().unwrap();
    assert_eq!(row.brief_path, Some(artifact.target.clone()));
    assert_eq!(row.briefed_at, None);
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    assert!(!format!("{receipt:?}").contains("Private"));
}

pub(crate) fn brief_completion_couples_marker_journal_and_history(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal),
) {
    let (run, row) = occasion(ledger);
    let artifact = artifact(&row, 10);
    saved(save(ledger, run, &row, None, &artifact));
    let receipt = saved(publish(ledger, run, &artifact, 11));
    assert!(receipt.completed.is_empty());
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger.occasion(&row.id).unwrap().unwrap().briefed_at,
        Some(UtcInstant { secs: 11 })
    );
    assert_eq!(
        ledger.brief(&row.id).unwrap().unwrap().published_at,
        Some(UtcInstant { secs: 11 })
    );
    assert!(matches!(
        receipt.records[0].fact,
        HistoryFact::BriefPublished { .. }
    ));
    let before = ledger.pending_exports().unwrap();
    assert!(matches!(
        publish(ledger, run, &artifact, 12),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_exports().unwrap(), before);
}

pub(crate) fn brief_rewrites_preserve_pin_and_refuse_stale_versions(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal),
) {
    let (run, row) = occasion(ledger);
    let old = artifact(&row, 10);
    saved(save(ledger, run, &row, None, &old));
    saved(publish(ledger, run, &old, 11));
    let row = ledger.occasion(&row.id).unwrap().unwrap();
    let mut next = artifact(&row, 12);
    next.first_written = old.first_written;
    for case in 0..3 {
        let mut changed = next.clone();
        let mut version = old.version;
        match case {
            0 => version.snapshot.0[0] ^= 1,
            1 => changed.target = "/briefs/renamed.md".into(),
            _ => changed.first_written.secs += 1,
        }
        assert!(
            matches!(
                save(ledger, run, &row, Some(version), &changed),
                HistoryWrite::Unconfirmed(_)
            ),
            "case {case}"
        );
    }
    saved(save(ledger, run, &row, Some(old.version), &next));
    assert_eq!(ledger.brief(&row.id).unwrap(), Some(next.clone()));
    assert_eq!(
        ledger.occasion(&row.id).unwrap().unwrap().briefed_at,
        Some(UtcInstant { secs: 11 })
    );
    saved(publish(ledger, run, &next, 13));
    assert_eq!(
        ledger.occasion(&row.id).unwrap().unwrap().briefed_at,
        Some(UtcInstant { secs: 13 })
    );
}

pub(crate) fn pending_briefs_require_explicit_completion_and_history_failure_rolls_back(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal),
) {
    let (run, row) = occasion(ledger);
    let artifact = artifact(&row, 10);
    let (delta, facts) = change(&row, None, &artifact);
    let mut facts = facts.to_vec();
    facts.push((
        UtcInstant { secs: 10 },
        HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.brief(&row.id).unwrap().is_none());
    assert_eq!(ledger.occasion(&row.id).unwrap(), Some(row.clone()));
    assert!(ledger.pending_publications().unwrap().is_empty());
    saved(save(ledger, run, &row, None, &artifact));
    assert!(ledger.clear_publication(&artifact.target).is_err());
    let row = ledger.occasion(&row.id).unwrap().unwrap();
    assert!(matches!(
        save(ledger, run, &row, Some(artifact.version), &artifact),
        HistoryWrite::Unconfirmed(_)
    ));
    let delta = WorkDelta {
        publication_clears: vec![artifact.target.clone()],
        ..Default::default()
    };
    assert!(matches!(
        ledger.commit_step(
            run,
            &delta,
            &[(
                UtcInstant { secs: 11 },
                HistoryFact::RunFinished(vpt_domain::run::RunOutcome::Completed)
            )]
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
}

macro_rules! cases {
    ($case:ident) => {
        $case!(brief_input_stream_keeps_published_notes_and_stops_on_visitor_failure);
        $case!(brief_targets_require_canonical_absolute_paths);
        $case!(brief_publication_journal_rejects_unrelated_writers);
        $case!(brief_history_cannot_claim_unsaved_or_unpublished_work);
        $case!(accepted_brief_pins_its_path_and_keeps_the_occasion_unbriefed);
        $case!(brief_completion_couples_marker_journal_and_history);
        $case!(brief_rewrites_preserve_pin_and_refuse_stale_versions);
        $case!(pending_briefs_require_explicit_completion_and_history_failure_rolls_back);
    };
}
pub(crate) use cases;

pub(crate) fn brief_history_cannot_claim_unsaved_or_unpublished_work(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal),
) {
    let (run, row) = occasion(ledger);
    let before = ledger.pending_exports().unwrap();
    for fact in [
        HistoryFact::BriefSaved {
            occasion: row.id.clone(),
        },
        HistoryFact::BriefPublished {
            occasion: row.id.clone(),
        },
    ] {
        assert!(matches!(
            ledger.commit_step(
                run,
                &WorkDelta::default(),
                &[(UtcInstant { secs: 10 }, fact)]
            ),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
}

pub(crate) fn brief_publication_journal_rejects_unrelated_writers(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal + RecordingLedger),
) {
    let (run, row) = occasion(ledger);
    let artifact = artifact(&row, 10);
    saved(save(ledger, run, &row, None, &artifact));
    let entry = ledger.pending_publications().unwrap().pop().unwrap();
    for completed in [false, true] {
        if completed {
            saved(publish(ledger, run, &artifact, 11));
        }
        let before = ledger.pending_publications().unwrap();
        let commit = LedgerCommit {
            publications: vec![entry.clone()],
            ..Default::default()
        };
        assert!(ledger.commit(&commit).is_err());
        assert!(matches!(
            ledger.commit_step(
                run,
                &WorkDelta {
                    commit,
                    ..Default::default()
                },
                &[(
                    UtcInstant { secs: 12 },
                    HistoryFact::RunFinished(vpt_domain::run::RunOutcome::Completed)
                )]
            ),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(ledger.pending_publications().unwrap(), before);
    }
}

pub(crate) fn brief_targets_require_canonical_absolute_paths(
    ledger: &(impl BriefLedger + OccasionLedger + RunHistory + PublicationJournal),
) {
    let (run, row) = occasion(ledger);
    for path in [
        "/briefs/./planned.md",
        "/briefs//planned.md",
        "/briefs/../planned.md",
    ] {
        let mut artifact = artifact(&row, 10);
        artifact.target = path.into();
        assert!(
            matches!(
                save(ledger, run, &row, None, &artifact),
                HistoryWrite::Unconfirmed(_)
            ),
            "{path}"
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert!(ledger.brief(&row.id).unwrap().is_none());
    }
}
