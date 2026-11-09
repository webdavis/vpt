use serde::Deserialize;
#[derive(Deserialize)]
pub(super) struct Object {
    pub oid: String,
    pub kind: String,
    pub subject: String,
    pub path: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub event: Option<Event>,
    pub task: Option<Task>,
}
#[derive(Deserialize)]
pub(super) struct Event {
    pub start: String,
    pub end: String,
    pub timezone: Option<String>,
    pub status: String,
    #[serde(default)]
    pub attendees: Vec<Attendee>,
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
#[derive(Deserialize)]
pub(super) struct Task {
    pub done: bool,
    pub priority: u32,
    pub due: Option<String>,
}
