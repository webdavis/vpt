mod settings;
pub use settings::{CheckerRuns, EngineConfig, EngineKind, EnginePair, EngineSettings};
mod selection;
pub use selection::{SelectedEngines, SelectionError};
