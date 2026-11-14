use super::{Report, output, work};
use crate::{
    cli::output::Outcome,
    compose::{
        errors::open_error,
        history::{commit, recover},
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*, review::alerts::ReviewQueue};
use vpt_domain::run::RunKind;
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    settings: &Settings,
    roots: &Roots,
    resend_refused: bool,
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
    let helper = HelperClient::new(settings.helper_path.clone());
    let mut warnings = BTreeSet::new();
    let mut publications = Vec::new();
    let mut report = Report::default();
    let mut notices = ReviewQueue::default();
    let result = (|| {
        recover(&ledger, &mut sink, &mut warnings)?;
        let run = commit(
            ledger.begin_run(RunKind::Pipeline, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let mut context = work::Context {
            ledger: &ledger,
            sink: &mut sink,
            warnings: &mut warnings,
            settings,
            roots,
            state: &state,
            helper: &helper,
            resend_refused,
        };
        let result = (|| {
            publications = super::retention::startup(&mut context, run, &mut report)?;
            work::run(&mut context, run, &mut report, &mut notices)
        })();
        work::finish(&mut context, run, &notices, &report, result)?;
        output::success(&report, false)
    })()
    .map_err(|mut error: Box<ErrorDocument>| {
        let mut seen: std::collections::HashSet<_> = error.completed.iter().cloned().collect();
        for id in report.completed() {
            if seen.insert(id.clone()) {
                error.completed.push(id);
            }
        }
        error.diagnostics.extend(report.diagnostics());
        error
            .diagnostics
            .extend(report.retention.diagnostics.iter().cloned());
        error.diagnostics.extend(
            report
                .retention
                .warnings
                .iter()
                .copied()
                .map(crate::compose::events::warning_text)
                .map(str::to_owned),
        );
        error
    });
    let mut outcome = crate::compose::events::attach(result, &warnings);
    outcome.add_completed(publications);
    Ok(outcome)
}
