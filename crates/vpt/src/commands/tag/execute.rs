mod steps;
mod writes;
use super::{
    Request,
    output::{self, Report},
    scan,
};
use crate::{
    cli::output::Outcome,
    compose::{
        errors::open_error, events::attach, history::recover, mentions::Context, publication,
        tag_vocabulary,
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock, known_tags};
use vpt_application::{
    Settings,
    ports::{Clock, VocabularyKind},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    settings: &Settings,
    roots: &Roots,
    request: &Request<'_>,
) -> Result<Outcome, Box<ErrorDocument>> {
    let state = crate::compose::mutation::state(roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    let ledger = SqliteLedger::open(&state).map_err(|error| Box::new(open_error(error)))?;
    let mut sink = LoggedEvents::new(
        &state,
        &ledger,
        &settings.report_command,
        &settings.alert_command,
        || SystemClock.now(),
    );
    let mut warnings = BTreeSet::new();
    let mut retention = None;
    let mut completed = Vec::new();
    let result = (|| {
        recover(&ledger, &mut sink, &mut warnings)?;
        let recovered = crate::compose::startup::repair(
            &ledger,
            &mut sink,
            &HelperClient::new(settings.helper_path.clone()),
            settings,
            roots,
            &mut warnings,
        )?;
        warnings.extend(recovered.retention.warnings.iter().copied());
        retention = Some(recovered);
        let context = Context::read(settings, roots)?;
        let (vocabulary, _) = tag_vocabulary::read(settings, context.index())?;
        let rename = vocabulary
            .rename(request.old, request.new)
            .map_err(super::usage)?;
        let carriers = scan::carriers(&ledger, &rename)?;
        let known = known_tags::prepare_renamed(&settings.tags.known_tags_path, &rename)
            .map_err(|error| publication::vocabulary::document(VocabularyKind::Tags, error))?;
        if carriers.is_empty() && !known.added() {
            return Ok(output::success(&Report {
                dry_run: false,
                recordings: &[],
                known_tags_changed: false,
                untouched: scan::untouched(roots, context.index(), &rename, &[])?,
            }));
        }
        let mut work = steps::Work {
            ledger: &ledger,
            sink: &mut sink,
            warnings: &mut warnings,
            settings,
            roots,
            context: &context,
        };
        let progress = work.run(&rename, &known, carriers, &mut completed)?;
        let index = crate::compose::index::read(settings, roots)?;
        let report = Report {
            dry_run: false,
            recordings: &progress.renamed,
            known_tags_changed: known.added(),
            untouched: scan::untouched(roots, &index, &rename, &[])?,
        };
        Ok(match progress.failed.is_empty() {
            true => output::success(&report),
            false => output::partial(&report, progress.failed),
        })
    })();
    let mut outcome = attach(result, &warnings);
    outcome.add_completed(completed);
    Ok(match retention {
        Some(retention) => crate::compose::startup::attach(outcome, retention),
        None => outcome,
    })
}
