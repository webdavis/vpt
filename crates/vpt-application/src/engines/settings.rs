use std::collections::BTreeMap;
use std::time::Duration;

mod debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckerRuns {
    Always,
    OnFailure,
}

#[derive(Clone, PartialEq, Eq)]
pub enum EngineKind {
    Apple,
    Whisply {
        command: Vec<String>,
        device: String,
        model: String,
    },
    Command {
        command: Vec<String>,
        family: String,
        local: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub name: String,
    pub kind: EngineKind,
}

impl EngineConfig {
    pub fn family(&self) -> &str {
        match &self.kind {
            EngineKind::Apple => "apple",
            EngineKind::Whisply { .. } => "whisper",
            EngineKind::Command { family, .. } => family,
        }
    }
    pub fn local(&self) -> bool {
        match &self.kind {
            EngineKind::Command { local, .. } => *local,
            _ => true,
        }
    }
    pub fn lexical_classification(&self) -> bool {
        !matches!(self.kind, EngineKind::Command { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnginePair {
    pub main: String,
    pub checker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSettings {
    pub language: String,
    pub pair: EnginePair,
    pub checker_runs: CheckerRuns,
    pub allow_same_family: bool,
    pub timeout: Duration,
    pub definitions: BTreeMap<String, EngineConfig>,
    pub by_language: BTreeMap<String, EnginePair>,
}

impl Default for EngineSettings {
    fn default() -> Self {
        let definitions = [
            EngineConfig {
                name: "apple".into(),
                kind: EngineKind::Apple,
            },
            EngineConfig {
                name: "whisply".into(),
                kind: EngineKind::Whisply {
                    command: vec!["whisply".into()],
                    device: "mlx".into(),
                    model: "large-v3-turbo".into(),
                },
            },
        ]
        .into_iter()
        .map(|engine| (engine.name.clone(), engine))
        .collect();
        Self {
            language: "en-US".into(),
            pair: EnginePair {
                main: "apple".into(),
                checker: None,
            },
            checker_runs: CheckerRuns::Always,
            allow_same_family: false,
            timeout: Duration::from_secs(1800),
            definitions,
            by_language: BTreeMap::new(),
        }
    }
}
