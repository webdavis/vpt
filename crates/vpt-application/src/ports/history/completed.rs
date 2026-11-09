use super::*;
impl WorkDelta {
    pub fn completed_recordings(&self) -> Vec<RecordingId> {
        let mut completed: Vec<_> = self
            .commit
            .recordings
            .iter()
            .map(|row| row.id.clone())
            .collect();
        completed
    }
}

impl super::HistoryReceipt {
    pub fn completed_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.completed.iter().map(ToString::to_string).collect();
        for record in &self.records {
            if let vpt_domain::run::HistoryFact::BriefPublished { occasion } = &record.fact {
                let id = occasion.to_string();
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        ids
    }
}
