use super::*;
use vpt_domain::context::ContextOccasion;
mod event;
mod model;
pub(super) struct Page {
    pub occasions: Vec<ContextOccasion>,
    pub next: Option<String>,
}
impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalendarPage")
            .field("count", &self.occasions.len())
            .finish_non_exhaustive()
    }
}
pub(super) fn decode(bytes: &[u8], calendar: &str) -> Result<Page, Failure> {
    let value = vpt_protocol::read_bounded(bytes, &vpt_protocol::Limits::incoming())
        .map_err(|_| Failure::Invalid)?;
    let reply: model::Reply = serde_json::from_value(value).map_err(|_| Failure::Invalid)?;
    if reply
        .kind
        .as_deref()
        .is_some_and(|s| s != "calendar#events")
        || reply
            .next_page_token
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.chars().any(char::is_control))
    {
        return Err(Failure::Invalid);
    }
    let mut snapshot = vpt_domain::context::ContextSnapshot::default();
    snapshot.status.calendar = vpt_domain::brief::ContextStatus::Present;
    for row in reply.items {
        if let Some(occasion) = event::convert(row, calendar, reply.time_zone.as_deref())? {
            snapshot.occasions.push(occasion);
        }
    }
    let text = vpt_protocol::context::encode(&snapshot).map_err(|_| Failure::Invalid)?;
    let snapshot = vpt_protocol::context::decode(text.as_bytes())
        .map_err(|_| Failure::Invalid)?
        .snapshot;
    Ok(Page {
        occasions: snapshot.occasions,
        next: reply.next_page_token,
    })
}
#[cfg(test)]
mod tests;
