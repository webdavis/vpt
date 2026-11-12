use serde::Deserialize;
#[derive(Deserialize)]
pub(super) struct Ls {
    pub objects: Vec<Object>,
}
#[derive(Deserialize)]
pub(super) struct Object {
    pub oid: String,
    pub kind: String,
    pub subject: String,
    pub path: String,
    pub task: Option<Task>,
}
#[derive(Deserialize)]
pub(super) struct Task {
    pub done: bool,
    pub priority: u32,
    pub due: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Agenda {
    pub events: Vec<Row>,
}
#[derive(Deserialize)]
pub(super) struct Row {
    pub oid: String,
    pub subject: String,
    pub path: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub status: String,
    pub start: String,
    pub end: String,
    pub timezone: Option<String>,
    #[serde(flatten)]
    pub people: People,
}
#[derive(Deserialize)]
pub(super) struct Shown {
    pub oid: String,
    pub kind: String,
    pub event: Option<People>,
}
#[derive(Deserialize)]
pub(super) struct People {
    pub attendees: Option<Vec<Attendee>>,
    pub organizer: Option<Person>,
}
#[derive(Deserialize)]
pub(super) struct Attendee {
    pub email: String,
}
#[derive(Deserialize)]
pub(super) struct Person {
    pub email: String,
    pub name: Option<String>,
}
