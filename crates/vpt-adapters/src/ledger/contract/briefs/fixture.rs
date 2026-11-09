use super::*;
use sha2::{Digest, Sha256};
use std::path::Path;
use vpt_domain::{
    brief::{self, ContextSummary, Options, Request, document::Snapshot, markdown},
    digest::Sha256Digest,
    history::RunId,
    occasion::OccasionKey,
    time::UtcOffset,
};

pub(crate) fn occasion(ledger: &(impl OccasionLedger + RunHistory)) -> (RunId, OccasionRecord) {
    let run = saved(ledger.begin_run(vpt_domain::run::RunKind::Brief, UtcInstant { secs: 1 })).run;
    let change = crate::ledger::occasion::prepare(
        ledger,
        OccasionKey::dam("Private provider id").unwrap(),
        crate::ledger::contract::occasions::details(),
        UtcInstant { secs: 2 },
    )
    .unwrap();
    let row = change.occasion.clone();
    saved(crate::ledger::contract::occasions::write(
        ledger, run, change,
    ));
    (run, row)
}
pub(crate) fn artifact(row: &OccasionRecord, at: i64) -> BriefArtifact {
    let generated_at = UtcInstant { secs: at };
    let pack = brief::assemble(&Request {
        occasion: &row.details,
        terms: &[],
        candidates: &[],
        now: generated_at,
        options: Options::default(),
    })
    .unwrap();
    let snapshot = Snapshot {
        id: row.id.clone(),
        source: row.key.source(),
        occasion: row.details.clone(),
        pack,
        context: ContextSummary::default(),
        tasks: vec![],
        generated_at,
    };
    let filename = markdown::filename(&snapshot.document(), 60).unwrap();
    let text = markdown::create(&markdown::Note {
        document: snapshot.document(),
        filename: &filename,
        origin: Path::new("/briefs"),
        wiki_root: None,
        link_style: vpt_domain::note::links::LinkStyle::Markdown,
        profile: vpt_domain::note::document::Profile::Portable,
    })
    .unwrap();
    let digest = |bytes: &[u8]| Sha256Digest(Sha256::digest(bytes).into());
    let version = BriefVersion {
        content: digest(text.as_bytes()),
        snapshot: digest(
            crate::ledger::brief::codec::encode(&snapshot)
                .unwrap()
                .as_bytes(),
        ),
    };
    BriefArtifact {
        snapshot,
        text,
        version,
        target: Path::new("/briefs").join(filename),
        managed_tags: row.details.tags.clone(),
        first_written: generated_at,
        write_offset: UtcOffset { secs: 0 },
        published_at: None,
    }
}
pub(crate) fn change(
    row: &OccasionRecord,
    expected: Option<BriefVersion>,
    artifact: &BriefArtifact,
) -> (WorkDelta, [(UtcInstant, HistoryFact); 1]) {
    (
        WorkDelta {
            brief: Some(BriefChange {
                expected_occasion: row.clone(),
                expected,
                artifact: artifact.clone(),
            }),
            commit: LedgerCommit {
                publications: vec![DirtyPublication {
                    target: artifact.target.clone(),
                    expected_previous: expected.map(|v| v.content),
                    intended: artifact.version.content,
                    recorded_at: artifact.snapshot.generated_at,
                }],
                ..Default::default()
            },
            ..Default::default()
        },
        [(
            artifact.snapshot.generated_at,
            HistoryFact::BriefSaved {
                occasion: row.id.clone(),
            },
        )],
    )
}
pub(crate) fn save(
    ledger: &impl RunHistory,
    run: RunId,
    row: &OccasionRecord,
    expected: Option<BriefVersion>,
    artifact: &BriefArtifact,
) -> HistoryWrite<HistoryReceipt> {
    let (delta, facts) = change(row, expected, artifact);
    ledger.commit_step(run, &delta, &facts)
}
pub(crate) fn publish(
    ledger: &impl RunHistory,
    run: RunId,
    artifact: &BriefArtifact,
    secs: i64,
) -> HistoryWrite<HistoryReceipt> {
    let at = UtcInstant { secs };
    ledger.commit_step(
        run,
        &WorkDelta {
            brief_published: Some(BriefPublication {
                occasion: artifact.snapshot.id.clone(),
                expected: artifact.version,
                at,
            }),
            publication_clears: vec![artifact.target.clone()],
            ..Default::default()
        },
        &[(
            at,
            HistoryFact::BriefPublished {
                occasion: artifact.snapshot.id.clone(),
            },
        )],
    )
}
