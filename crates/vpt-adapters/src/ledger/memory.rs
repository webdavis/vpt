mod briefs;
mod briefs_inputs;
mod eligibility;
mod engine_outputs;
mod expirations;
mod flags;
mod notes;
mod occasions;
mod proposals;
mod publications;
mod recordings;
mod relations;
mod retention;
mod retention_artifacts;
mod tags;
mod transcript;
mod verification;
mod vocabulary;
use std::sync::Mutex;
use vpt_application::ports::{
    DirtyPublication, LedgerCommit, LedgerError, NewIntent, RecordingLedger, RecordingRecord,
    RetentionIntent, RetentionJournal, SeenRow,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

mod history;

#[derive(Clone, Default)]
struct State {
    expirations: Vec<vpt_application::ports::RetentionArtifact>,
    releases: Vec<vpt_application::ports::ReleaseRecord>,
    briefs: Vec<vpt_application::ports::BriefArtifact>,
    occasions: Vec<vpt_application::ports::OccasionRecord>,
    verification: Vec<super::verification::state::VerifiedArtifact>,
    proposals: Vec<vpt_application::ports::AcceptedProposal>,
    ambiguities: Vec<(RecordingId, Vec<vpt_domain::relations::MentionAmbiguity>)>,
    relations: Vec<(RecordingId, Vec<vpt_domain::relations::Relation>)>,
    tags: Vec<(RecordingId, Vec<vpt_domain::tags::Tag>)>,
    vocabulary: Vec<vpt_application::ports::VocabularyPublication>,
    notes: Vec<vpt_application::ports::NoteArtifact>,
    flags: Vec<(RecordingId, vpt_domain::review::ReviewFlag)>,
    engine_outputs: Vec<vpt_application::ports::EngineArtifact>,
    transcripts: Vec<vpt_application::ports::AcceptedTranscript>,
    seen: Vec<SeenRow>,
    recordings: Vec<RecordingRecord>,
    publications: Vec<DirtyPublication>,
    intents: Vec<(RetentionIntent, Option<UtcInstant>)>,
    next_intent_id: i64,
    history: history::HistoryState,
}

#[derive(Default)]
pub struct MemoryLedger {
    state: Mutex<Box<State>>,
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self::default()
    }
}

fn with<T>(
    ledger: &MemoryLedger,
    operation: impl FnOnce(&mut State) -> T,
) -> Result<T, LedgerError> {
    let mut state = ledger
        .state
        .lock()
        .map_err(|_| LedgerError::Io("in-memory ledger unavailable".into()))?;
    Ok(operation(&mut state))
}

fn upsert_seen(state: &mut State, row: &SeenRow) {
    match state.seen.iter_mut().find(|stored| stored.path == row.path) {
        Some(stored) => {
            let first_seen = stored.first_seen;
            *stored = row.clone();
            stored.first_seen = first_seen;
        }
        None => state.seen.push(row.clone()),
    }
}

fn upsert_publication(state: &mut State, entry: &DirtyPublication) -> Result<(), LedgerError> {
    if state
        .vocabulary
        .iter()
        .any(|row| row.entry.target == entry.target)
    {
        return Err(super::vocabulary::conflict());
    }
    match state
        .publications
        .iter_mut()
        .find(|stored| stored.target == entry.target)
    {
        Some(stored) => *stored = entry.clone(),
        None => state.publications.push(entry.clone()),
    }
    Ok(())
}

fn apply_commit(state: &mut State, batch: &LedgerCommit) -> Result<(), LedgerError> {
    for row in &batch.seen {
        upsert_seen(state, row);
    }
    for entry in &batch.publications {
        upsert_publication(state, entry)?;
    }
    for recording in &batch.recordings {
        if state
            .recordings
            .iter()
            .any(|stored| stored.id == recording.id || stored.digest == recording.digest)
        {
            return Err(LedgerError::Conflict(
                "digest or identity already recorded".into(),
            ));
        }
        state.recordings.push(recording.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
