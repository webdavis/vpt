use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Document {
    pub context: Summary,
    pub occasions: Vec<Occasion>,
    pub tasks: Vec<Task>,
}
#[derive(Deserialize)]
pub(super) struct Summary {
    pub calendar: Status,
    pub tasks: Status,
}
#[derive(Deserialize)]
pub(super) struct Status {
    pub status: String,
    pub reason: Option<String>,
    pub detail: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Occasion {
    pub source: String,
    pub id: String,
    pub calendar: Option<String>,
    pub title: String,
    pub at: String,
    pub duration_secs: Option<u64>,
    pub participants: Vec<Participant>,
    #[serde(default)]
    pub tags: Vec<String>,
}
#[derive(Deserialize)]
pub(super) struct Participant {
    pub name: String,
    pub email: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Task {
    pub source: String,
    pub project: String,
    pub id: String,
    pub content: String,
    pub due: Option<String>,
    pub completed: bool,
    pub priority: Option<u32>,
}
