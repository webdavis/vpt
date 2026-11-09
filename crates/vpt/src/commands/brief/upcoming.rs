use super::*;
use crate::compose::{history::commit, publication};
use std::collections::BTreeSet;
use vpt_adapters::{SqliteLedger, SystemClock};
use vpt_application::ports::*;
use vpt_domain::{brief::ContextStatus, context::ContextSnapshot, run::HistoryFact};

pub(super) fn validate(settings: &Settings) -> Result<(), Box<ErrorDocument>> {
    if !settings.brief.trigger_enabled {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "brief.trigger.enabled must be true for --upcoming",
        )));
    }
    context::validate(settings)?;
    if settings.context.source == vpt_application::ContextSource::None {
        return Err(discovery_error());
    }
    Ok(())
}

fn discovery_error() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Command,
        "upcoming occasion discovery failed or is unavailable",
    ))
}

fn discover(settings: &Settings) -> Result<ContextSnapshot, Box<ErrorDocument>> {
    let start = SystemClock.now();
    let lead = i64::try_from(settings.brief.lead_time.as_secs()).map_err(|_| invalid())?;
    let end = vpt_domain::time::UtcInstant {
        secs: start.secs.checked_add(lead).ok_or_else(invalid)?,
    };
    let snapshot = context::collect_window(settings, ContextWindow { start, end })?;
    if snapshot.status.calendar != ContextStatus::Present {
        return Err(discovery_error());
    }
    Ok(snapshot)
}

pub(super) fn write(
    ledger: &SqliteLedger,
    settings: &Settings,
    roots: &Roots,
    run: vpt_domain::history::RunId,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
    completed: &mut Vec<String>,
) -> Result<Outcome, Box<ErrorDocument>> {
    let context = discover(settings)?;
    for event in &context.occasions {
        let at = SystemClock.now();
        let change = vpt_adapters::prepare_occasion(
            ledger,
            event.key().map_err(|_| invalid())?,
            event.details.clone(),
            at,
        )
        .map_err(ledger_error)?;
        let row = change.occasion.clone();
        if row.briefed_at.is_some() {
            if change.expected.as_ref() != Some(&row) {
                commit(
                    ledger.commit_step(
                        run,
                        &WorkDelta {
                            occasion: Some(change),
                            ..Default::default()
                        },
                        &[(at, HistoryFact::OccasionSaved { occasion: row.id })],
                    ),
                    sink,
                    warnings,
                )?;
            }
            continue;
        }
        let prepared = prepare::assemble(ledger, settings, roots, row, Some(change), &context)?;
        roots.create_leaves().map_err(|error| {
            crate::compose::config_refusal::in_run(ledger, sink, run, error, warnings)
        })?;
        commit(
            ledger.commit_step(run, &prepared.delta, &prepared.facts),
            sink,
            warnings,
        )?;
        completed.extend(publication::repair_upcoming(
            ledger,
            &stores(roots)?,
            settings,
            sink,
            warnings,
        )?);
    }
    Ok(Outcome::Success {
        document: vpt_protocol::result::document(
            "brief",
            serde_json::json!({"written": completed}),
        ),
        human: format!("{} briefs written\n", completed.len()),
        completed: completed.clone(),
        diagnostics: vec![],
    })
}
