use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreKey {
    Audio,
    Transcripts,
    Analysis,
    Briefs,
    EngineOutputs,
    Drafts,
    Released,
}

impl StoreKey {
    pub fn all() -> [Self; 7] {
        [
            Self::Audio,
            Self::Transcripts,
            Self::Analysis,
            Self::Briefs,
            Self::EngineOutputs,
            Self::Drafts,
            Self::Released,
        ]
    }

    pub fn key_name(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Transcripts => "transcripts",
            Self::Analysis => "analysis",
            Self::Briefs => "briefs",
            Self::EngineOutputs => "engine_outputs",
            Self::Drafts => "drafts",
            Self::Released => "released",
        }
    }

    pub fn default_leaf(self) -> &'static str {
        match self {
            Self::EngineOutputs => "engine-outputs",
            other => other.key_name(),
        }
    }

    pub fn from_key_name(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|key| key.key_name() == name)
    }

    pub fn is_private(self) -> bool {
        self != Self::Released
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootName {
    Home,
    Store(StoreKey),
    State,
    Config,
    VoiceMemos,
}

impl RootName {
    pub fn key_name(self) -> String {
        match self {
            Self::Home => "home.path".into(),
            Self::Store(store) => format!("stores.{}", store.key_name()),
            Self::State => "home.state_dir".into(),
            Self::Config => "config".into(),
            Self::VoiceMemos => "source.recordings_dir".into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RootConflict {
    pub first: RootName,
    pub second: RootName,
}

pub fn check_overlaps(roots: &[(RootName, &Path)]) -> Result<(), RootConflict> {
    for (index, &(first, first_path)) in roots.iter().enumerate() {
        for &(second, second_path) in &roots[index + 1..] {
            if forbidden(first, first_path, second, second_path) {
                return Err(RootConflict { first, second });
            }
        }
    }
    Ok(())
}

fn forbidden(first: RootName, a: &Path, second: RootName, b: &Path) -> bool {
    match (first, second) {
        (RootName::Home | RootName::Config, RootName::VoiceMemos) => a.starts_with(b),
        (RootName::VoiceMemos, RootName::Home | RootName::Config) => b.starts_with(a),
        (RootName::Store(_), RootName::Store(_))
        | (RootName::VoiceMemos, RootName::Store(_) | RootName::State)
        | (RootName::Store(_) | RootName::State, RootName::VoiceMemos)
        | (RootName::Store(StoreKey::Released), RootName::State | RootName::Config)
        | (RootName::State | RootName::Config, RootName::Store(StoreKey::Released)) => {
            a.starts_with(b) || b.starts_with(a)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
