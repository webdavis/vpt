use crate::cli::output::Outcome;
use crate::style::{self, Verdict};
use serde_json::{Value, json};
use vpt_protocol::result::document;

mod row;
pub(crate) use row::Row;

pub(crate) struct Check {
    pub(crate) name: &'static str,
    pub(crate) blurb: &'static str,
    pub(crate) rows: Vec<Row>,
    pub(crate) skipped: Option<&'static str>,
}

impl Check {
    fn verdict(&self) -> Verdict {
        let has = |wanted| self.rows.iter().any(|row| row.verdict == wanted);
        if has(Verdict::Issue) {
            Verdict::Issue
        } else if has(Verdict::Warning) {
            Verdict::Warning
        } else if has(Verdict::Ok) {
            Verdict::Ok
        } else {
            Verdict::NotRun
        }
    }

    fn json(&self) -> Value {
        json!({"name": self.name, "result": self.verdict().name(), "detail": self.blurb,
            "advice": self.skipped, "rows": self.rows.iter().map(Row::json).collect::<Vec<_>>()})
    }

    fn human(&self) -> String {
        let mut text = style::heading(self.name, self.blurb);
        if let Some(reason) = self.skipped {
            text.push_str(&style::mark(Verdict::NotRun, "not run"));
            text.push_str(&style::advice(reason));
        }
        for row in &self.rows {
            text.push_str(&style::row(row.verdict, &row.name, &row.detail));
            text.extend(row.advice.as_deref().map(style::advice));
        }
        text
    }
}

pub(crate) fn outcome(command: &str, checks: &[Check], fixed: Option<Vec<Value>>) -> Outcome {
    let attention: Vec<(&Check, &Row)> = [Verdict::Issue, Verdict::Warning]
        .into_iter()
        .flat_map(|wanted| {
            checks.iter().flat_map(move |check| {
                check
                    .rows
                    .iter()
                    .filter(move |row| row.verdict == wanted)
                    .map(move |row| (check, row))
            })
        })
        .collect();
    let needs_attention: Vec<Value> = attention
        .iter()
        .map(|(check, row)| {
            json!({"check": check.name, "name": row.name, "result": row.verdict.name(),
                "detail": row.detail, "advice": row.advice})
        })
        .collect();
    let mut body = json!({"version": env!("CARGO_PKG_VERSION"),
        "checks": checks.iter().map(Check::json).collect::<Vec<_>>(),
        "needs_attention": needs_attention});
    if let Some(fixed) = fixed {
        body["fixed"] = Value::Array(fixed);
    }
    let document = document(command, body);
    let entries: Vec<(Verdict, String)> = attention
        .iter()
        .map(|(check, row)| {
            (
                row.verdict,
                format!("{} {}: {}", check.name, row.name, row.detail),
            )
        })
        .collect();
    let human = std::iter::once(style::version_line())
        .chain(checks.iter().map(Check::human))
        .chain([style::needs_attention(&entries)])
        .collect();
    if entries
        .iter()
        .any(|(verdict, _)| *verdict == Verdict::Issue)
    {
        Outcome::Findings { document, human }
    } else {
        Outcome::Success {
            document,
            human,
            completed: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}
