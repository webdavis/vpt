#[derive(Clone, Copy)]
pub(super) enum Profile {
    General,
    Engine,
}

#[derive(Clone, Copy)]
pub(super) enum Location {
    Root,
    Claims,
    Segments,
    Segment,
    Words,
    Transcriptions,
    Transcription,
    Chunks,
    Chunk,
    ChunkWords,
    Other,
}

impl Location {
    pub(super) fn field(self, key: &str) -> Self {
        match (self, key) {
            (Self::Root, "summary" | "actions") => Self::Claims,
            (Self::Root, "segments") => Self::Segments,
            (Self::Segment, "words") => Self::Words,
            (Self::Root, "transcription") => Self::Transcriptions,
            (Self::Transcriptions, _) => Self::Transcription,
            (Self::Transcription, "chunks") => Self::Chunks,
            (Self::Chunk, "words") => Self::ChunkWords,
            _ => Self::Other,
        }
    }

    pub(super) fn element(self) -> Self {
        match self {
            Self::Segments => Self::Segment,
            Self::Chunks => Self::Chunk,
            _ => Self::Other,
        }
    }

    pub(super) fn limit(self, profile: Profile, general: usize) -> usize {
        match (profile, self) {
            (Profile::Engine, Self::Segments | Self::Words) => 65_536,
            _ => general,
        }
    }
}
