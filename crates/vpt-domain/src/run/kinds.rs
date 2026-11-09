#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunKind {
    Redact,
    Brief,
    Pipeline,
    Review,
    VerifyNote,
    Synthesize,
    Confirm,
    Note,
    Transcribe,
    Ingest,
    Recover,
    Publish,
    Retain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    Failed,
    Interrupted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkOutcome {
    Completed,
    Failed,
    Unknown,
    Deferred,
}
