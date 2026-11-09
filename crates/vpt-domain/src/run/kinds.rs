#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunKind {
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
