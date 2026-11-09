use super::*;
use vpt_domain::occasion::time::OccasionTime;
use vpt_domain::occasion::{OccasionDetails, OccasionSource, Participant};

pub(super) fn convert(
    row: model::Event,
    calendar: &str,
    zone: Option<&str>,
) -> Result<Option<ContextOccasion>, Failure> {
    match row.status.as_str() {
        "cancelled" => return Ok(None),
        "confirmed" | "tentative" => (),
        _ => return Err(Failure::Invalid),
    }
    let (at, start) = when(row.start.ok_or(Failure::Invalid)?, zone)?;
    let (_, end) = when(row.end.ok_or(Failure::Invalid)?, zone)?;
    let nanos = end.as_nanosecond() - start.as_nanosecond();
    if nanos < 0 {
        return Err(Failure::Invalid);
    }
    let duration = u64::try_from(nanos / 1_000_000_000).map_err(|_| Failure::Invalid)?;
    let mut participants = Vec::<Participant>::new();
    for person in row.attendees.into_iter().chain(row.organizer) {
        let participant = Participant {
            name: person.display_name.unwrap_or_default(),
            email: person.email,
        };
        if let Some(existing) = participants.iter_mut().find(|p| {
            (participant.email.is_some() && p.email == participant.email) || **p == participant
        }) {
            if existing.name.is_empty() {
                existing.name = participant.name;
            }
        } else {
            participants.push(participant);
        }
    }
    Ok(Some(ContextOccasion {
        source: OccasionSource::Google,
        id: row.id,
        calendar: Some(calendar.into()),
        details: OccasionDetails {
            at,
            duration_secs: Some(duration),
            title: row
                .summary
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Untitled event".into()),
            participants,
            tags: vec![],
        },
    }))
}
fn when(
    value: model::When,
    calendar_zone: Option<&str>,
) -> Result<(OccasionTime, jiff::Timestamp), Failure> {
    match (value.date, value.date_time) {
        (Some(date), None) => {
            let zone = value
                .time_zone
                .as_deref()
                .or(calendar_zone)
                .ok_or(Failure::Invalid)?;
            if date.len() != 10 {
                return Err(Failure::Invalid);
            }
            crate::context::time::parse(&date, Some(zone))
        }
        (None, Some(text)) => {
            if text.contains('[') {
                return Err(Failure::Invalid);
            }
            match value.time_zone {
                Some(zone) => crate::context::time::parse(&format!("{text}[{zone}]"), None),
                None => crate::context::time::parse(&text, None),
            }
        }
        _ => Err(Failure::Invalid),
    }
}
