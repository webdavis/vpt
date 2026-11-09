use crate::layout::StoreKey;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetentionCounts([u32; 7]);

impl RetentionCounts {
    pub fn get(self, store: StoreKey) -> u32 {
        self.0[index(store)]
    }

    pub fn set(&mut self, store: StoreKey, count: u32) {
        self.0[index(store)] = count;
    }
}

fn index(store: StoreKey) -> usize {
    match store {
        StoreKey::Audio => 0,
        StoreKey::Transcripts => 1,
        StoreKey::Analysis => 2,
        StoreKey::Briefs => 3,
        StoreKey::EngineOutputs => 4,
        StoreKey::Drafts => 5,
        StoreKey::Released => 6,
    }
}
