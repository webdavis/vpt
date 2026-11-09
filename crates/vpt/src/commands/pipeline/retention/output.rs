use crate::cli::output::Outcome;
use vpt_application::RetentionReport;

pub(crate) fn attach(mut outcome: Outcome, report: &RetentionReport, dry_run: bool) -> Outcome {
    match crate::commands::retention::output::outcome(Ok(report.clone()), dry_run) {
        Outcome::Success {
            mut document,
            human,
            diagnostics,
            ..
        } => {
            if let Some(fields) = document.as_object_mut() {
                fields.remove("schema");
                fields.remove("command");
            }
            if let Outcome::Success {
                document: combined,
                human: summary,
                diagnostics: notices,
                ..
            } = &mut outcome
            {
                combined["retention"] = document;
                summary.push_str(&human);
                notices.extend(diagnostics);
            }
            outcome
        }
        mut failure => {
            if let Outcome::Success { completed, .. } = outcome {
                failure.add_completed(completed);
            }
            failure
        }
    }
}
