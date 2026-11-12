use super::*;
use vpt_domain::{
    context::ContextOccasion,
    occasion::{OccasionDetails, Participant},
};

#[derive(Debug)]
pub(in crate::context::dam) struct Event {
    pub occasion: ContextOccasion,
    pub attended: bool,
}

pub(super) fn convert(row: model::Row) -> Result<Option<Event>, Failure> {
    match row.status.as_str() {
        "cancelled" => return Ok(None),
        "confirmed" | "tentative" => (),
        _ => return Err(Failure::Invalid),
    }
    let (at, start) = time::parse(&row.start, row.timezone.as_deref())?;
    let (_, end) = time::parse(&row.end, row.timezone.as_deref())?;
    let nanos = end.as_nanosecond() - start.as_nanosecond();
    let duration_secs = u64::try_from(nanos / 1_000_000_000).map_err(|_| Failure::Invalid)?;
    if nanos < 0 {
        return Err(Failure::Invalid);
    }
    let attended = row.people.attendees.is_some();
    Ok(Some(Event {
        occasion: ContextOccasion {
            source: OccasionSource::Dam,
            id: row.oid,
            calendar: None,
            details: OccasionDetails {
                at,
                title: row.subject,
                duration_secs: Some(duration_secs),
                participants: participants(row.people)?,
                tags: row.labels,
            },
        },
        attended,
    }))
}

pub(super) fn participants(people: model::People) -> Result<Vec<Participant>, Failure> {
    let mut participants = Vec::<Participant>::new();
    for attendee in people.attendees.unwrap_or_default() {
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
    if let Some(person) = people.organizer {
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
    Ok(participants)
}
