use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteProfile {
    Portable,
    Obsidian,
}

pub use vpt_domain::note::links::LinkStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSettings {
    pub profile: NoteProfile,
    pub link_style: LinkStyle,
    pub slug_max_chars: u32,
    pub obsidian: ObsidianSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsidianSettings {
    pub vault_root: Option<PathBuf>,
    pub hub_transcripts: String,
    pub hub_analysis: String,
    pub hub_briefs: String,
}

impl NoteSettings {
    pub fn wiki_root<'a>(&'a self, stores: &'a super::StorePaths) -> Option<&'a std::path::Path> {
        if self.profile == NoteProfile::Obsidian {
            return self.obsidian.vault_root.as_deref();
        }
        let mut root = stores.transcripts.as_path();
        while !stores.analysis.starts_with(root) || !stores.briefs.starts_with(root) {
            root = root.parent()?;
        }
        Some(root)
    }
}
