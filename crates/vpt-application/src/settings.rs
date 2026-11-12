use std::path::{Path, PathBuf};
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::Hold;
mod brief;
mod context;
pub use context::{ContextSettings, ContextSource, GoogleCredentials};
mod note;
pub use brief::BriefSettings;
mod share;
mod synthesis;
pub use note::{LinkStyle, NoteProfile, NoteSettings, ObsidianSettings};
pub use share::ShareSettings;
pub use synthesis::SynthesisSettings;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub config_version: u32,
    pub engines: crate::engines::EngineSettings,
    pub reconcile: ReconcileSettings,
    pub readability_enabled: bool,
    pub note: NoteSettings,
    pub tags: TagSettings,
    pub relations: vpt_domain::relations::Options,
    pub synthesis: SynthesisSettings,
    pub brief: BriefSettings,
    pub context: ContextSettings,
    pub share: ShareSettings,
    pub handoff_allow_private: bool,
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub symlink_target: Option<PathBuf>,
    pub helper_path: PathBuf,
    pub stores: StorePaths,
    pub source: SourceSettings,
    pub report_command: Vec<String>,
    pub alert_command: Vec<String>,
    pub import_extract_command: Vec<String>,
    pub alert_aggregate_after: u32,
    pub retention: RetentionSettings,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconcileSettings {
    pub options: vpt_domain::uncertainty::Options,
    pub known_terms_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSettings {
    pub options: vpt_domain::tags::Options,
    pub known_tags_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    pub audio: PathBuf,
    pub transcripts: PathBuf,
    pub analysis: PathBuf,
    pub briefs: PathBuf,
    pub engine_outputs: PathBuf,
    pub drafts: PathBuf,
    pub released: PathBuf,
}

impl StorePaths {
    pub fn get(&self, key: StoreKey) -> &Path {
        match key {
            StoreKey::Audio => &self.audio,
            StoreKey::Transcripts => &self.transcripts,
            StoreKey::Analysis => &self.analysis,
            StoreKey::Briefs => &self.briefs,
            StoreKey::EngineOutputs => &self.engine_outputs,
            StoreKey::Drafts => &self.drafts,
            StoreKey::Released => &self.released,
        }
    }

    pub fn set(&mut self, key: StoreKey, path: PathBuf) {
        match key {
            StoreKey::Audio => self.audio = path,
            StoreKey::Transcripts => self.transcripts = path,
            StoreKey::Analysis => self.analysis = path,
            StoreKey::Briefs => self.briefs = path,
            StoreKey::EngineOutputs => self.engine_outputs = path,
            StoreKey::Drafts => self.drafts = path,
            StoreKey::Released => self.released = path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSettings {
    pub recordings_dir: PathBuf,
    pub read_titles: bool,
    pub quiet_period_secs: u64,
    pub deferral_page_threshold: u32,
    pub max_audio_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionSettings {
    pub enabled: bool,
    pub include_audio: bool,
    pub holds: Vec<(StoreKey, Hold)>,
}

impl RetentionSettings {
    pub fn hold(&self, key: StoreKey) -> Hold {
        self.holds
            .iter()
            .find(|(store, _)| *store == key)
            .map_or(Hold::never(), |(_, hold)| *hold)
    }
}
