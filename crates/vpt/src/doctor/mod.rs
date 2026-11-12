mod checks;
mod engines;
mod probes;
mod source;
mod vault;

use crate::cli::output::Outcome;
use crate::compose::Environment;
use std::path::Path;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

const INSTALL_HINT: &str = "install with `cargo install --git https://github.com/webdavis/vpt vpt`, then \
`swift build -c release` in helper/vpt-macos and copy .build/release/vpt-macos beside vpt";

pub(crate) fn run(environment: &Environment, config: Option<&Path>) -> Outcome {
    let checks = checks::all(environment, config);
    let human = checks
        .iter()
        .map(|check| {
            format!(
                "{} {:<26} {}\n",
                match (check.ok, check.notice) {
                    (false, _) => "FAIL",
                    (true, true) => "note",
                    (true, false) => "ok  ",
                },
                check.name,
                check.detail
            )
        })
        .collect();
    let failed = checks.iter().filter(|check| !check.ok).count();
    if failed == 0 {
        return Outcome::Success {
            completed: Vec::new(),
            diagnostics: Vec::new(),
            document: document("doctor", serde_json::json!({"checks": checks})),
            human,
        };
    }
    let error = ErrorDocument::new(ErrorKind::Refused, format!("{failed} checks failed"))
        .rule("doctor_checks")
        .checks(checks);
    Outcome::FailedReport { error, human }
}
