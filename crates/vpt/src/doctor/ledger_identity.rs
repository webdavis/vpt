use super::report::Row;
use crate::compose::ledger_identity::{ADVICE, happened, observe, rule};
use crate::style::Verdict;
use std::path::{Component, Path};
use vpt_adapters::Markers;

pub(super) fn rows(folder: &Path, markers: Option<&Markers>) -> Vec<Row> {
    let mut rows = vec![identity(folder, markers)];
    rows.extend(conflict_copies(folder));
    rows.extend(synced_folder(folder));
    rows
}

fn identity(folder: &Path, markers: Option<&Markers>) -> Row {
    let Some(markers) = markers else {
        return Row::not_run("identity", "this machine's marker folder is unknown");
    };
    let observed = match observe(folder, markers) {
        Ok(observed) => observed,
        Err(error) => return Row::issue("identity", error.message),
    };
    match (observed.finding(), &observed.marker) {
        (Some(finding), _) => Row::issue(
            "identity",
            format!("{}: {}", rule(finding), happened(finding)),
        )
        .advice(ADVICE),
        (None, Some(_)) => Row::ok("identity", "this machine's last write is in this ledger"),
        (None, None) => Row::ok(
            "identity",
            "this machine has not written to this ledger yet",
        ),
    }
}

fn conflict_copies(folder: &Path) -> Option<Row> {
    let mut copies: Vec<String> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("vpt") && name.ends_with(".db") && name != "vpt.db")
        .collect();
    copies.sort();
    (!copies.is_empty()).then(|| {
        Row::new(
            "conflict_copies",
            Verdict::Warning,
            format!(
                "beside vpt.db, files that look like copies a sync service made: {}",
                copies.join(", ")
            ),
        )
        .advice("vpt reads only vpt.db; a copy may hold another machine's writes, so keep it until you know")
    })
}

/// Detects a synced folder by its path alone.
fn synced_folder(folder: &Path) -> Option<Row> {
    let names: Vec<_> = folder
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    let synced = names.contains(&"Dropbox")
        || names.windows(2).any(|pair| {
            pair[0] == "Library" && (pair[1] == "Mobile Documents" || pair[1] == "CloudStorage")
        });
    synced.then(|| {
        Row::ok(
            "synced_folder",
            "the ledger folder sits inside a folder a sync service manages; when it delivers \
             another machine's ledger, an older copy or half a set, writing commands refuse and \
             read-only commands warn, and `vpt ledger show` lists the facts",
        )
    })
}
