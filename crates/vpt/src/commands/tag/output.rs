use crate::cli::output::Outcome;
use std::path::PathBuf;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

#[derive(Clone, Copy)]
pub(super) struct Untouched {
    pub briefs: usize,
    pub notes: usize,
}

pub(super) struct Report<'a> {
    pub dry_run: bool,
    pub recordings: &'a [(RecordingId, Vec<PathBuf>)],
    pub known_tags_changed: bool,
    pub untouched: Untouched,
}

pub(super) fn success(report: &Report<'_>) -> Outcome {
    let changed = report.known_tags_changed || !report.recordings.is_empty();
    let rows: Vec<_> = report
        .recordings
        .iter()
        .map(|(id, notes)| serde_json::json!({"id": id.as_str(), "notes": notes}))
        .collect();
    Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: vpt_protocol::result::document(
            "tag rename",
            serde_json::json!({
                "dry_run": report.dry_run,
                "changed": changed,
                "recordings": rows,
                "known_tags_changed": report.known_tags_changed,
                "untouched_briefs": report.untouched.briefs,
                "untouched_notes": report.untouched.notes,
            }),
        ),
        human: human(report, changed),
    }
}

pub(super) fn partial(
    report: &Report<'_>,
    failed: Vec<(RecordingId, Box<ErrorDocument>)>,
) -> Outcome {
    let first = &failed[0].1;
    let mut error = ErrorDocument::new(
        first.kind,
        format!(
            "the tag was not renamed on {}; run the same command again to finish",
            count(failed.len(), "recording")
        ),
    )
    .ids(failed.iter().map(|(id, _)| id.to_string()).collect());
    error.rule.clone_from(&first.rule);
    Outcome::FailedReport {
        error,
        human: human(report, true),
    }
}

fn human(report: &Report<'_>, changed: bool) -> String {
    let notes: usize = report.recordings.iter().map(|(_, notes)| notes.len()).sum();
    let mut human = match (changed, report.dry_run) {
        (false, _) => "No recording or known tag carries that tag; nothing changed\n".to_owned(),
        (true, preview) => format!(
            "{} the tag on {} and {}{}\n",
            if preview { "Would rename" } else { "Renamed" },
            count(report.recordings.len(), "recording"),
            count(notes, "note"),
            if report.known_tags_changed {
                " and in known tags"
            } else {
                ""
            },
        ),
    };
    for (_, paths) in report.recordings {
        for path in paths {
            human.push_str(&format!("  {}\n", path.display()));
        }
    }
    let Untouched { briefs, notes } = report.untouched;
    human.push_str(&format!(
        "{} the old tag and were left alone, because a brief's tags are its meeting's labels\n\
         {} the old tag and were left alone\n",
        carry(briefs, "brief"),
        carry(notes, "other note"),
    ));
    human
}

fn count(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

fn carry(n: usize, noun: &str) -> String {
    format!(
        "{} {}",
        count(n, noun),
        if n == 1 { "carries" } else { "carry" }
    )
}
