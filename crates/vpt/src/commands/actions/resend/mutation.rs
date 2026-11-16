use crate::{
    cli::output::Outcome,
    compose::{
        actions::{Counts, Delivery},
        errors::open_error,
        events::attach,
        history::{commit, recover},
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{
    Settings,
    ports::{Clock, RunHistory, WorkDelta},
};
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, RunKind},
};
use vpt_protocol::{error::ErrorDocument, result::document};

/// Makes the refused actions in scope pending again and sends them, under the run kind the
/// commands that resent them before recorded: analyze for one recording, run for all, so a ledger
/// written here stays readable by an older vpt.
pub(super) fn run(
    settings: &Settings,
    roots: &Roots,
    only: Option<&RecordingId>,
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
        let kind = only.map_or(RunKind::Pipeline, |_| RunKind::Analyze);
        let run = commit(
            ledger.begin_run(kind, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let mut delivery = Delivery {
            ledger: &ledger,
            settings,
            sink: &mut sink,
            warnings: &mut warnings,
            run,
        };
        let sent = delivery
            .resend_refused(only)
            .and_then(|()| delivery.send(only));
        let finished = HistoryFact::RunFinished(crate::compose::run_outcome::from_result(&sent));
        commit(
            ledger.commit_step(run, &WorkDelta::default(), &[(SystemClock.now(), finished)]),
            &mut sink,
            &mut warnings,
        )?;
        sent.map(success)
    })();
    let outcome = attach(result, &warnings);
    Ok(match retention {
        Some(retention) => crate::compose::startup::attach(outcome, retention),
        None => outcome,
    })
}

fn success(counts: Counts) -> Outcome {
    let mut human = format!(
        "sent {}, failed {}, refused {}",
        counts.sent, counts.failed, counts.refused
    );
    if let Some(withdrawn) = counts.withdrawn {
        human.push_str(&format!(", withdrawn {withdrawn}"));
    }
    Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        human: human + "\n",
        document: document(
            "actions resend",
            serde_json::json!({"preview": false, "actions": counts.json()}),
        ),
    }
}
