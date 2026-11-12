use super::*;
use vpt_domain::{history::SafeFailureClass, sweep::DeferralReason};

pub(super) fn encode(value: EventDetail) -> Value {
    match value {
        EventDetail::ReviewAggregate { recordings } => {
            json!({"tag":"review_aggregate","recordings":recordings})
        }
        EventDetail::Brief {
            notes,
            spans,
            tasks,
            unresolved,
        } => {
            json!({"tag":"brief","notes":notes,"spans":spans,"tasks":tasks,"unresolved":unresolved})
        }
        EventDetail::None => json!({"tag":"none"}),
        EventDetail::Review {
            spans,
            agreed_unverified,
        } => {
            json!({"tag":"review","spans":spans,"agreed_unverified":agreed_unverified})
        }
        EventDetail::Failure(class) => json!({"tag":"failure","class":failure_tag(class)}),
        EventDetail::Deferred { reason, count } => {
            json!({"tag":"deferred","reason":reason.as_str(),"count":count})
        }
        EventDetail::Retention { completed, failed } => {
            json!({"tag":"retention","completed":completed,"failed":failed})
        }
        EventDetail::Actions {
            sent,
            failed,
            refused,
        } => {
            json!({"tag":"actions","sent":sent,"failed":failed,"refused":refused})
        }
    }
}

fn failure_tag(class: SafeFailureClass) -> &'static str {
    match class {
        SafeFailureClass::SourceUnavailable => "source_unavailable",
        SafeFailureClass::ArchiveFailed => "archive_failed",
        SafeFailureClass::RecordingFailed => "recording_failed",
        SafeFailureClass::LedgerUnavailable => "ledger_unavailable",
        SafeFailureClass::ExportUnavailable => "export_unavailable",
        SafeFailureClass::ConfigurationRefused => "configuration_refused",
    }
}

pub(super) fn decode(value: &Value) -> Result<EventDetail, LedgerError> {
    match string(value, "tag")? {
        "review_aggregate" => Ok(EventDetail::ReviewAggregate {
            recordings: count(value, "recordings")?,
        }),
        "brief" => Ok(EventDetail::Brief {
            notes: count(value, "notes")?,
            spans: count(value, "spans")?,
            tasks: count(value, "tasks")?,
            unresolved: count(value, "unresolved")?,
        }),
        "none" => Ok(EventDetail::None),
        "review" => Ok(EventDetail::Review {
            spans: count(value, "spans")?,
            agreed_unverified: count(value, "agreed_unverified")?,
        }),
        "failure" => {
            let class = match string(value, "class")? {
                "source_unavailable" => SafeFailureClass::SourceUnavailable,
                "archive_failed" => SafeFailureClass::ArchiveFailed,
                "recording_failed" => SafeFailureClass::RecordingFailed,
                "ledger_unavailable" => SafeFailureClass::LedgerUnavailable,
                "export_unavailable" => SafeFailureClass::ExportUnavailable,
                "configuration_refused" => SafeFailureClass::ConfigurationRefused,
                _ => return Err(corrupt()),
            };
            Ok(EventDetail::Failure(class))
        }
        "deferred" => Ok(EventDetail::Deferred {
            reason: DeferralReason::parse(string(value, "reason")?).ok_or_else(corrupt)?,
            count: count(value, "count")?,
        }),
        "retention" => Ok(EventDetail::Retention {
            completed: count(value, "completed")?,
            failed: count(value, "failed")?,
        }),
        "actions" => Ok(EventDetail::Actions {
            sent: count(value, "sent")?,
            failed: count(value, "failed")?,
            refused: count(value, "refused")?,
        }),
        _ => Err(corrupt()),
    }
}
