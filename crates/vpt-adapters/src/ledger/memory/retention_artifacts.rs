use super::{MemoryLedger, State, with};
use crate::ledger::retention_artifacts::{Inventory, MAX_ARTIFACTS, invalid};
use std::collections::{HashMap, HashSet};
use vpt_application::ports::*;

impl RetentionArtifacts for MemoryLedger {
    fn retention_artifacts(&self) -> Result<Vec<RetentionArtifact>, LedgerError> {
        with(self, |state| read(state))?
    }
}

pub(super) fn read(state: &State) -> Result<Vec<RetentionArtifact>, LedgerError> {
    if state.publications.len() > MAX_ARTIFACTS {
        return Err(invalid());
    }
    let pending: HashSet<_> = state
        .publications
        .iter()
        .map(|row| row.target.as_path())
        .collect();
    let mut inventory = Inventory::default();
    let status = |path: &std::path::Path, expired: bool| {
        if pending.contains(path) {
            RetentionStatus::PublicationPending
        } else if expired {
            RetentionStatus::Expired
        } else {
            RetentionStatus::Published
        }
    };
    for row in &state.recordings {
        inventory.push(
            ArtifactKind::Audio,
            row.id.as_str(),
            &row.audio_path,
            row.digest,
            status(&row.audio_path, row.audio_trashed_at.is_some()),
            row.audio_trashed_at,
        )?;
    }
    let recordings: HashMap<_, _> = state
        .recordings
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect();
    for row in &state.engine_outputs {
        inventory.push(
            ArtifactKind::EngineOutput,
            row.recording.as_str(),
            &row.target,
            row.digest,
            status(&row.target, false),
            None,
        )?;
    }
    for row in &state.notes {
        let record = recordings.get(row.recording.as_str()).ok_or_else(invalid)?;
        let (kind, expired) = match row.stage {
            NoteStage::Transcript => (
                ArtifactKind::Transcript,
                record.stages.note == StageState::Expired,
            ),
            NoteStage::Analysis => (
                ArtifactKind::Analysis,
                record.stages.synthesis == StageState::Expired,
            ),
        };
        inventory.push(
            kind,
            row.recording.as_str(),
            &row.target,
            row.digest,
            status(&row.target, expired),
            None,
        )?;
    }
    for row in &state.briefs {
        let status = if row.published_at.is_none() {
            RetentionStatus::PublicationPending
        } else {
            status(&row.target, false)
        };
        inventory.push(
            ArtifactKind::Brief,
            row.snapshot.id.as_str(),
            &row.target,
            row.version.content,
            status,
            None,
        )?;
    }
    for row in &state.releases {
        let status = if row.finalized_at.is_some() {
            RetentionStatus::Published
        } else {
            RetentionStatus::PublicationPending
        };
        for (kind, path, digest) in [
            (ArtifactKind::Release, &row.destination, row.version.content),
            (
                ArtifactKind::RedactionReport,
                &row.report_path,
                row.version.report,
            ),
        ] {
            inventory.push(kind, row.recording.as_str(), path, digest, status, None)?;
        }
    }
    let mut rows = inventory.finish()?;
    super::expirations::overlay(state, &mut rows)?;
    Ok(rows)
}
