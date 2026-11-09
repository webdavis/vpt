use super::*;
use ureq::http::{HeaderValue, Request as HttpRequest};
pub(super) fn build(request: Request<'_>) -> Result<HttpRequest<Vec<u8>>, Failure> {
    match request {
        Request::Refresh(value) => {
            let body = pairs(&[
                ("grant_type", "refresh_token"),
                ("client_id", &value.client_id),
                ("client_secret", &value.client_secret),
                ("refresh_token", &value.refresh_token),
                ("scope", token::SCOPE),
            ]);
            HttpRequest::builder()
                .method("POST")
                .uri("https://oauth2.googleapis.com/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body.into_bytes())
                .map_err(|_| Failure::Invalid)
        }
        Request::Events {
            token,
            calendar,
            window,
            page,
        } => {
            let start = window.start.rfc3339();
            let end = window.end.rfc3339();
            let mut query = vec![
                ("singleEvents", "true"),
                ("timeMin", &start),
                ("timeMax", &end),
                ("maxResults", "256"),
                ("showDeleted", "false"),
                ("orderBy", "startTime"),
                (
                    "fields",
                    "kind,timeZone,nextPageToken,items(id,status,summary,start,end,attendees(email,displayName),organizer(email,displayName))",
                ),
            ];
            if let Some(page) = page {
                query.push(("pageToken", page));
            }
            let uri = format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events?{}",
                encode(calendar),
                pairs(&query)
            );
            let mut authorization =
                HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Failure::Invalid)?;
            authorization.set_sensitive(true);
            HttpRequest::builder()
                .method("GET")
                .uri(uri)
                .header("Authorization", authorization)
                .body(vec![])
                .map_err(|_| Failure::Invalid)
        }
    }
}
fn pairs(values: &[(&str, &str)]) -> String {
    values
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}
fn encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            result.push(char::from(byte));
        } else {
            result.push('%');
            result.push(char::from(HEX[(byte >> 4) as usize]));
            result.push(char::from(HEX[(byte & 15) as usize]));
        }
    }
    result
}
