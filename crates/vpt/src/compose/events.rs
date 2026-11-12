use vpt_application::ports::{
    EventSink, HistoryReceipt, SinkError, SinkOutcome, SinkStop, SinkWarning,
};
use vpt_domain::run::RecoveryView;

pub(crate) struct Observation;

impl EventSink for Observation {
    fn after_commit(&mut self, _: &HistoryReceipt) -> SinkOutcome {
        stopped()
    }
    fn recover(&mut self, _: &RecoveryView) -> SinkOutcome {
        stopped()
    }
}

fn stopped() -> SinkOutcome {
    SinkOutcome::Stop(Box::new(SinkStop {
        reason: SinkError::UnsafeState,
        committed: None,
        warnings: Default::default(),
    }))
}

pub(crate) fn warning_text(warning: SinkWarning) -> &'static str {
    match warning {
        SinkWarning::ExportUnavailable => "event log export is unavailable",
        SinkWarning::ExportProgressUnconfirmed => "event log export acknowledgement is unconfirmed",
        SinkWarning::ExportFailureEvidenceUnconfirmed => {
            "event log failure evidence is unconfirmed"
        }
        SinkWarning::ExportBookkeepingPostCommitFailure => {
            "event log bookkeeping failed after commit"
        }
        SinkWarning::HookFailed => "configured notification command failed",
        SinkWarning::ActionFailed => "configured action command failed",
        SinkWarning::ActionRefused => "configured action command refused an action",
    }
}

pub(crate) fn attach_error(
    mut error: Box<vpt_protocol::error::ErrorDocument>,
    warnings: &std::collections::BTreeSet<SinkWarning>,
) -> Box<vpt_protocol::error::ErrorDocument> {
    for warning in warnings {
        let message = warning_text(*warning);
        if !error.diagnostics.iter().any(|text| text == message) {
            error.diagnostics.push(message.to_owned());
        }
    }
    error
}

pub(crate) fn attach(
    result: Result<crate::cli::output::Outcome, Box<vpt_protocol::error::ErrorDocument>>,
    warnings: &std::collections::BTreeSet<SinkWarning>,
) -> crate::cli::output::Outcome {
    use crate::cli::output::Outcome;
    match result {
        Ok(mut outcome) => {
            if let Outcome::Success {
                document,
                human,
                diagnostics,
                ..
            } = &mut outcome
            {
                document["warnings"] = serde_json::json!(
                    warnings
                        .iter()
                        .copied()
                        .map(warning_text)
                        .collect::<Vec<_>>()
                );
                for warning in warnings {
                    diagnostics.push(warning_text(*warning).to_owned());
                    human.push_str(&format!("Warning: {}\n", warning_text(*warning)));
                }
            }
            outcome
        }
        Err(error) => Outcome::Failure(*attach_error(error, warnings)),
    }
}

#[cfg(test)]
mod tests;
