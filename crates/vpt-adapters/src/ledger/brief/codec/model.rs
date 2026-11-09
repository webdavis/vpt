use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub schema: String,
    pub id: String,
    pub source: String,
    pub occasion: Occasion,
    pub generated_at: i64,
    pub context: Context,
    pub tasks: Vec<Task>,
    pub pack: Pack,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Occasion {
    pub at: String,
    pub title: String,
    pub duration_secs: Option<u64>,
    pub participants: Vec<(String, Option<String>)>,
    pub tags: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pack {
    pub participants: Vec<Participant>,
    pub selected: Vec<usize>,
    pub decisions: Vec<Decision>,
    pub omitted_notes: usize,
    pub rejected_notes: usize,
    pub omitted_spans: usize,
    pub items: Vec<Quote>,
    pub sources: Vec<Source>,
    pub unresolved: Unresolved,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Participant {
    pub participant: usize,
    pub term: String,
    pub matched_note: Option<String>,
    pub confirmation: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Decision {
    pub recording: String,
    pub selected: bool,
    pub rejected: Option<String>,
    pub matched: Option<Match>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Match {
    pub selector: String,
    pub values: Vec<String>,
    pub anchors: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Quote {
    pub text: String,
    pub at_secs: f64,
    pub source: Source,
    pub certainty: String,
    pub selector: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub recording: String,
    pub note: String,
    pub digest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Unresolved {
    pub flags: usize,
    pub notes_with_flags: usize,
    pub reviewed: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    pub calendar: Half,
    pub tasks: Half,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Half {
    pub status: String,
    pub reason: Option<String>,
    pub detail: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Task {
    pub source: String,
    pub project: String,
    pub id: String,
    pub content: String,
    pub due: Option<String>,
    pub completed: bool,
    pub priority: Option<u32>,
}
