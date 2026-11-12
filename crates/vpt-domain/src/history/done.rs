use super::*;

impl Event {
    pub fn recording_done(
        kind: EventKind,
        recording: RecordingId,
        note: Option<PathBuf>,
        at: UtcInstant,
    ) -> Self {
        Self {
            kind,
            state: EventState::Done,
            recording: Some(recording),
            occasion: None,
            detail: EventDetail::None,
            counts: EventCounts::default(),
            paths: note
                .into_iter()
                .map(|path| EventPath {
                    key: EventPathKey::Note,
                    path,
                })
                .collect(),
            occurred_at: at,
        }
    }
}
