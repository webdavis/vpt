use super::*;
use crate::cli::args::brief::Target;
use vpt_adapters::SystemClock;
use vpt_application::ports::*;
use vpt_domain::{
    brief::{self, document::Snapshot},
    occasion::OccasionKey,
    run::HistoryFact,
};

pub(super) struct Prepared {
    pub artifact: BriefArtifact,
    pub delta: WorkDelta,
    pub facts: Vec<(vpt_domain::time::UtcInstant, HistoryFact)>,
}
pub(super) fn run(
    ledger: &(impl OccasionLedger + BriefLedger + BriefInputsLedger),
    settings: &Settings,
    roots: &Roots,
    options: &Options,
) -> Result<Prepared, Box<ErrorDocument>> {
    let at = SystemClock.now();
    let (row, occasion) = match &options.target {
        Target::Existing(id) => (
            ledger
                .occasion(id)
                .map_err(ledger_error)?
                .ok_or_else(invalid)?,
            None,
        ),
        Target::Manual(details) => {
            let key = OccasionKey::manual(&details.at, &details.title).map_err(|_| invalid())?;
            let change = vpt_adapters::prepare_occasion(ledger, key, details.clone(), at)
                .map_err(ledger_error)?;
            (change.occasion.clone(), Some(change))
        }
        Target::Upcoming => return Err(invalid()),
    };
    let context = super::context::collect(settings, &row.details)?;
    assemble(ledger, settings, roots, row, occasion, &context)
}
pub(super) fn assemble(
    ledger: &(impl BriefLedger + BriefInputsLedger),
    settings: &Settings,
    roots: &Roots,
    row: OccasionRecord,
    occasion: Option<OccasionChange>,
    context: &vpt_domain::context::ContextSnapshot,
) -> Result<Prepared, Box<ErrorDocument>> {
    let at = SystemClock.now();
    let previous = ledger.brief(&row.id).map_err(ledger_error)?;
    let (terms, candidates) = super::sources::collect(ledger, settings, roots)?;
    let pack = brief::assemble(&brief::Request {
        occasion: &row.details,
        terms: &terms,
        candidates: &candidates,
        now: at,
        options: settings.brief.options,
    })
    .map_err(|_| invalid())?;
    let snapshot = Snapshot {
        id: row.id.clone(),
        source: row.key.source(),
        occasion: row.details.clone(),
        pack,
        context: context.status.clone(),
        tasks: context.tasks.clone(),
        generated_at: at,
    };
    let (artifact, expected_previous) = render::run(
        snapshot,
        previous.as_ref(),
        row.brief_path.as_deref(),
        settings,
        roots,
    )?;
    let mut facts = Vec::new();
    if let Some(change) = &occasion {
        facts.push((
            change.at,
            HistoryFact::OccasionSaved {
                occasion: row.id.clone(),
            },
        ));
    }
    facts.push((
        at,
        HistoryFact::BriefSaved {
            occasion: row.id.clone(),
        },
    ));
    let delta = WorkDelta {
        occasion,
        brief: Some(BriefChange {
            expected_occasion: row,
            expected: previous.map(|row| row.version),
            artifact: artifact.clone(),
        }),
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: artifact.target.clone(),
                expected_previous,
                intended: artifact.version.content,
                recorded_at: at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    Ok(Prepared {
        artifact,
        delta,
        facts,
    })
}
