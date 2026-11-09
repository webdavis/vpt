use super::*;
#[derive(Debug, Clone, PartialEq)]
pub struct Occurrence {
    pub words: Range<usize>,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlagView {
    pub identifier: Sha256Digest,
    pub flag: LexicalFlag,
    pub occurrences: Vec<Occurrence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Review {
    pub recording: RecordingId,
    pub transcribed: bool,
    pub flags: Vec<FlagView>,
    pub findings: Vec<(Sha256Digest, vpt_domain::review::VerificationFlag)>,
    pub diagnostics: Vec<(Sha256Digest, Diagnostic)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReviewError {
    NotFound,
    InvalidFlags,
    Ledger(LedgerError),
}
