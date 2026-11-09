use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Reply {
    pub kind: Option<String>,
    pub time_zone: Option<String>,
    pub next_page_token: Option<String>,
    pub items: Vec<Event>,
}
#[derive(Deserialize)]
pub(super) struct Event {
    pub id: String,
    #[serde(default = "confirmed")]
    pub status: String,
    pub summary: Option<String>,
    pub start: Option<When>,
    pub end: Option<When>,
    #[serde(default)]
    pub attendees: Vec<Person>,
    pub organizer: Option<Person>,
}
fn confirmed() -> String {
    "confirmed".into()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct When {
    pub date: Option<String>,
    pub date_time: Option<String>,
    pub time_zone: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Person {
    pub email: Option<String>,
    pub display_name: Option<String>,
}
