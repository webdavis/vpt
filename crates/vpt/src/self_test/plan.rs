use crate::report::{Check, Row};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) struct Part<'a> {
    pub(super) name: &'static str,
    pub(super) blurb: &'static str,
    pub(super) run: Option<&'a dyn Fn() -> Vec<Row>>,
}

pub(super) fn checks(parts: Vec<Part<'_>>, named: bool) -> Result<Vec<Check>, Box<ErrorDocument>> {
    let missing: Vec<&str> = parts
        .iter()
        .filter(|part| part.run.is_none())
        .map(|part| part.name)
        .collect();
    if named && !missing.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Usage,
            format!(
                "nothing to test for {}; see `vpt doctor {}`",
                missing.join(", "),
                missing.join(" ")
            ),
        )));
    }
    let tested = parts.len() - missing.len();
    let mut checks: Vec<Check> = parts
        .into_iter()
        .map(|part| Check {
            name: part.name,
            blurb: part.blurb,
            rows: part.run.map_or_else(
                || {
                    vec![
                        Row::not_run("configured", "not configured, so not tested")
                            .advice(format!("see `vpt doctor {}`", part.name)),
                    ]
                },
                |run| run(),
            ),
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
