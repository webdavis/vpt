use super::checks::Context;
use super::parts::{available, fix, not_tested};
use crate::report::Row;
use crate::style::Verdict;
use vpt_application::engines::EngineKind;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let settings = &context.settings.engines;
    let selected = settings.selected();
    settings
        .definitions
        .iter()
        .map(|(name, engine)| {
            let used = selected.contains(name);
            let (available, locality) = match &engine.kind {
                EngineKind::Apple => (context.helper().verdict == Verdict::Ok, "local"),
                EngineKind::Whisply { command, .. } => (available(command), "local"),
                EngineKind::Command { command, local, .. } => (
                    available(command),
                    if *local {
                        "declared local"
                    } else {
                        "declared remote (audio leaves the machine)"
                    },
                ),
            };
            let detail = format!(
                "{}, {}, {locality}",
                if used { "selected" } else { "unused" },
                if available {
                    "available"
                } else {
                    "unavailable"
                },
            );
            if used && !available {
                let fix = if matches!(engine.kind, EngineKind::Apple) {
                    "see `vpt doctor helper`".to_string()
                } else {
                    fix(&format!("engines.{name}"))
                };
                Row::issue(name.as_str(), detail).advice(fix)
            } else {
                Row::ok(name.as_str(), detail)
            }
        })
        .chain([not_tested("engines")])
        .collect()
}
