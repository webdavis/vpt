#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    Busy,
    UnsupportedSchema(u32),
    Conflict(String),
    Corrupt(String),
    Io(String),
    PathEscape(std::path::PathBuf),
}
