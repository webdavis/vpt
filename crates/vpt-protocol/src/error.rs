use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Refused,
    Usage,
    Config,
    Engine,
    Helper,
    Command,
    Store,
    Ledger,
}

impl ErrorKind {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Refused => 3,
            Self::Usage | Self::Config => 2,
            Self::Engine | Self::Helper | Self::Command | Self::Store | Self::Ledger => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Usage => "usage",
            Self::Config => "config",
            Self::Engine => "engine",
            Self::Helper => "helper",
            Self::Command => "command",
            Self::Store => "store",
            Self::Ledger => "ledger",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDocument {
    /// Execution status for run history; omitted from the public error document.
    pub interrupted: bool,
    pub kind: ErrorKind,
    pub rule: Option<String>,
    pub message: String,
    pub ids: Vec<String>,
    pub completed: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl ErrorDocument {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            interrupted: false,
            kind,
            rule: None,
            message: message.into(),
            ids: vec![],
            completed: vec![],
            diagnostics: vec![],
        }
    }
    pub fn rule(mut self, rule: &str) -> Self {
        self.rule = Some(rule.into());
        self
    }
    pub fn ids(mut self, ids: Vec<String>) -> Self {
        self.ids = ids;
        self
    }
    pub fn completed(mut self, completed: Vec<String>) -> Self {
        self.completed = completed;
        self
    }
    pub fn diagnostics(mut self, diagnostics: Vec<String>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
    pub fn exit_code(&self) -> i32 {
        self.kind.exit_code()
    }
    pub fn to_json(&self) -> Value {
        let mut error = serde_json::Map::new();
        error.insert("kind".into(), self.kind.name().into());
        error.insert(
            "rule".into(),
            self.rule.clone().map_or(Value::Null, Value::String),
        );
        error.insert("message".into(), self.message.clone().into());
        error.insert("ids".into(), self.ids.clone().into());
        error.insert("completed".into(), self.completed.clone().into());
        if !self.diagnostics.is_empty() {
            error.insert("diagnostics".into(), self.diagnostics.clone().into());
        }
        let mut document = serde_json::Map::new();
        document.insert("schema".into(), "vpt.error/1".into());
        document.insert("error".into(), Value::Object(error));
        Value::Object(document)
    }
}

#[cfg(test)]
mod tests;
