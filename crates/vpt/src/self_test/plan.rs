use crate::report::{Check, Row};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) type Run<'a> = Box<dyn Fn() -> Vec<Row> + 'a>;

/// `run` holds the test, or the `not_run` row that says why the part cannot be tested and names
/// the config key that would let it.
pub(super) struct Part<'a> {
    pub(super) name: &'static str,
    pub(super) blurb: &'static str,
    pub(super) run: Result<Run<'a>, Row>,
}

pub(super) fn checks(parts: Vec<Part<'_>>, named: bool) -> Result<Vec<Check>, Box<ErrorDocument>> {
    let refused: Vec<String> = parts
        .iter()
        .filter_map(|part| {
            let reason = part.run.as_ref().err()?;
            Some(format!("{} ({})", part.name, reason.detail))
        })
        .collect();
    if named && !refused.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Usage,
            format!("nothing to test for {}", refused.join(", ")),
        )));
    }
    let tested = parts.len() - refused.len();
    let mut checks: Vec<Check> = parts
        .into_iter()
        .map(|part| Check {
            name: part.name,
            blurb: part.blurb,
            rows: match part.run {
                Ok(run) => run(),
                Err(reason) => {
                    let advice = format!("see `vpt doctor {}`", part.name);
                    vec![Row {
                        advice: reason.advice.or(Some(advice)),
                        ..reason
                    }]
                }
            },
            skipped: None,
        })
        .collect();
    if tested == 0 {
        checks.push(Check {
            name: "run",
            blurb: "what this run tested",
            rows: vec![
                Row::issue("parts", "no part is configured, so nothing was tested")
                    .advice("run `vpt doctor` to see what to configure"),
            ],
            skipped: None,
        });
    }
    Ok(checks)
}
