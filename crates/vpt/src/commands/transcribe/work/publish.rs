use super::{Context, publish_output::Published};
use crate::{
    commands::note,
    compose::{errors::ledger_error, history::commit},
};
use vpt_adapters::SystemClock;
use vpt_application::{
    ports::*,
    review::{self, alerts::ReviewNotice},
};
use vpt_domain::{
    history::{EventCounts, RunId},
    identity::RecordingId,
    run::{HistoryFact, RunOutcome},
};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    id: &RecordingId,
    run: RunId,
) -> Result<Published, Box<ErrorDocument>> {
    let result = (|| {
        crate::compose::publication::repair(
            context.ledger,
            context.stores,
            context.settings,
            context.sink,
            context.warnings,
        )?;
        let inputs = context
            .ledger
            .note_inputs(id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(note::errors::not_found)?;
        let review = review::review(context.ledger, id).map_err(|_| {
            Box::new(ErrorDocument::new(
                ErrorKind::Ledger,
                "published review state is unavailable",
            ))
        })?;
        if inputs.record.stages.note == StageState::Expired {
            return Ok((
                Published {
                    note: None,
                    counts: review::alerts::counts(&review),
                },
                None,
            ));
        }
        let loaded;
        let mentions = match context.mentions {
            Some(mentions) => mentions,
            None => {
                loaded = crate::compose::mentions::Context::read(context.settings, context.roots)?;
                &loaded
            }
        };
        let target = note::write::Writer {
            mentions,
            ledger: context.ledger,
            stores: context.stores,
            settings: context.settings,
            options: vpt_application::note::WriteOptions {
                analysis_content: None,
                roots: &context.roots.stores,
                regenerate_expired: false,
            },
            sink: context.sink,
            warnings: context.warnings,
        }
        .write(inputs, run, &mut false)?;
        let notice = ReviewNotice::from_review(&review, target.clone());
        let counts = notice
            .as_ref()
            .map_or_else(EventCounts::default, |notice| notice.counts);
        Ok::<_, Box<ErrorDocument>>((
            Published {
                note: Some(target),
                counts,
            },
            notice,
        ))
    })();
    let at = SystemClock.now();
    let mut facts = Vec::new();
    if let Ok((_, Some(notice))) = &result
        && let Some(queue) = context.notices.as_deref_mut()
    {
        queue.push(notice.clone()).map_err(|_| {
            Box::new(ErrorDocument::new(
                ErrorKind::Ledger,
                "review alert capacity exceeded",
            ))
        })?;
    }
    if let Ok((_, Some(notice))) = &result
        && context.notices.is_none()
    {
        let events = review::alerts::events(
            std::slice::from_ref(notice),
            context.settings.alert_aggregate_after,
            at,
        )
        .map_err(|_| {
            Box::new(ErrorDocument::new(
                ErrorKind::Ledger,
                "review alert capacity exceeded",
            ))
        })?;
        facts.extend(
            events
                .into_iter()
                .map(|event| (at, HistoryFact::Event(event))),
        );
    }
    facts.push((
        at,
        HistoryFact::RunFinished(if result.is_ok() {
            RunOutcome::Completed
        } else {
            RunOutcome::Failed
        }),
    ));
    commit(
        context
            .ledger
            .commit_step(run, &WorkDelta::default(), &facts),
        context.sink,
        context.warnings,
    )?;
    result.map(|(published, _)| published)
}
