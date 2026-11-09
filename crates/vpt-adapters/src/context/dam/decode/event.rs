use super::*;
use vpt_domain::{
    context::ContextOccasion,
    occasion::{OccasionDetails, Participant},
};

pub(super) fn convert(row: model::Object) -> Result<Option<ContextOccasion>, Failure> {
    let event = row.event.ok_or(Failure::Invalid)?;
    match event.status.as_str() {
        "cancelled" => return Ok(None),
        "confirmed" | "tentative" => (),
        _ => return Err(Failure::Invalid),
    }
    let (at, start) = time::parse(&event.start, event.timezone.as_deref())?;
    let (_, end) = time::parse(&event.end, event.timezone.as_deref())?;
    let nanos = end.as_nanosecond() - start.as_nanosecond();
    let duration_secs = u64::try_from(nanos / 1_000_000_000).map_err(|_| Failure::Invalid)?;
    if nanos < 0 {
        return Err(Failure::Invalid);
    }
    let mut participants = Vec::<Participant>::new();
    for attendee in event.attendees {
        if attendee.email.trim().is_empty() {
            return Err(Failure::Invalid);
        }
        if !participants
            .iter()
            .any(|p| p.email.as_ref() == Some(&attendee.email))
        {
            participants.push(Participant {
                name: String::new(),
                email: Some(attendee.email),
            });
        }
    }
    if let Some(person) = event.organizer {
        let name = person.name.unwrap_or_default();
        if let Some(existing) = participants
            .iter_mut()
            .find(|p| p.email.as_ref() == Some(&person.email))
        {
            existing.name = name;
        } else {
            participants.push(Participant {
                name,
                email: Some(person.email),
            });
        }
    }
    Ok(Some(ContextOccasion {
        source: OccasionSource::Dam,
        id: row.oid,
        calendar: None,
        details: OccasionDetails {
            at,
            title: row.subject,
            duration_secs: Some(duration_secs),
            participants,
            tags: row.labels,
        },
    }))
}
