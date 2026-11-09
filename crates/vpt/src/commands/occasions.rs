use crate::cli::output::Outcome;
use serde_json::json;
use vpt_application::ports::OccasionLedger;
use vpt_domain::occasion::OccasionId;
use vpt_protocol::{
    error::{ErrorDocument, ErrorKind},
    result::document,
};

pub(crate) fn run(ledger: &impl OccasionLedger) -> Outcome {
    let rows = match ledger.occasions() {
        Ok(rows) => rows,
        Err(error) => return Outcome::Failure(crate::compose::errors::ledger_error(error)),
    };
    let values: Vec<_> = rows.iter().map(|row| json!({
        "id": row.id.as_str(), "at": row.details.at.as_str(), "title": row.details.title,
        "source": row.key.source().as_str(), "duration_secs": row.details.duration_secs,
        "participants": row.details.participants.iter().map(|p| json!({"name": p.name, "email": p.email})).collect::<Vec<_>>(),
        "tags": row.details.tags, "brief_path": row.brief_path,
        "briefed_at": row.briefed_at.map(|at| at.rfc3339()),
    })).collect();
    let human = if rows.is_empty() {
        "no occasions\n".into()
    } else {
        rows.iter()
            .map(|row| {
                format!(
                    "{}  {}  {}  {}\n",
                    row.id,
                    row.details.at.as_str(),
                    if row.briefed_at.is_some() {
                        "briefed"
                    } else {
                        "unbriefed"
                    },
                    row.details.title.escape_debug()
                )
            })
            .collect()
    };
    Outcome::Success {
        document: document("occasions", json!({"occasions": values})),
        human,
        completed: vec![],
        diagnostics: vec![],
    }
}

pub(crate) fn path(ledger: &impl OccasionLedger, id: &str, stage: &str) -> Outcome {
    let Ok(id) = OccasionId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid occasion id"));
    };
    if stage != "brief" {
        return Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            "occasion stage must be brief",
        ));
    }
    match ledger.occasion(&id) {
        Ok(Some(row)) if row.brief_path.is_some() => {
            let path = row.brief_path.expect("matched saved pin");
            Outcome::Success {
                document: document("path", json!({"path": path})),
                human: format!("{}\n", path.display()),
                completed: vec![],
                diagnostics: vec![],
            }
        }
        Ok(_) => Outcome::Failure(
            ErrorDocument::new(ErrorKind::Usage, "no pinned brief path for this occasion")
                .ids(vec![id.to_string()]),
        ),
        Err(error) => Outcome::Failure(crate::compose::errors::ledger_error(error)),
    }
}
