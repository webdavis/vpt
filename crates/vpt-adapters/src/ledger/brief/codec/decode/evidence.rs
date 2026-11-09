use super::*;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId};

pub(super) fn pack(raw: model::Pack) -> Result<EvidencePack, LedgerError> {
    Ok(EvidencePack {
        selection: Selection {
            participants: raw
                .participants
                .into_iter()
                .map(participant)
                .collect::<Result<_, _>>()?,
            selected: raw.selected,
            decisions: raw
                .decisions
                .into_iter()
                .map(decision)
                .collect::<Result<_, _>>()?,
            omitted_notes: raw.omitted_notes,
            rejected_notes: raw.rejected_notes,
        },
        items: raw.items.into_iter().map(quote).collect::<Result<_, _>>()?,
        sources: raw
            .sources
            .into_iter()
            .map(source)
            .collect::<Result<_, _>>()?,
        omitted_spans: raw.omitted_spans,
        unresolved: Unresolved {
            flags: raw.unresolved.flags,
            notes_with_flags: raw.unresolved.notes_with_flags,
            reviewed: raw.unresolved.reviewed,
        },
    })
}
fn participant(raw: model::Participant) -> Result<ParticipantMatch, LedgerError> {
    let term = match raw.term.as_str() {
        "confirmed" => TermState::Confirmed,
        "unconfirmed" => TermState::Unconfirmed,
        "missing-note" => TermState::MissingNote,
        "ambiguous-note" => TermState::AmbiguousNote,
        _ => return Err(invalid()),
    };
    Ok(ParticipantMatch {
        participant: raw.participant,
        term,
        matched_note: raw.matched_note.map(Into::into),
        confirmation: raw.confirmation,
    })
}
fn decision(raw: model::Decision) -> Result<Decision, LedgerError> {
    let rejected = raw
        .rejected
        .as_deref()
        .map(|s| match s {
            "missing-transcript" => Ok(Rejection::MissingTranscript),
            "no-confirmed-match" => Ok(Rejection::NoMatch),
            "outside-lookback" => Ok(Rejection::OutsideLookback),
            "future-capture" => Ok(Rejection::FutureCapture),
            _ => Err(invalid()),
        })
        .transpose()?;
    let matched = raw
        .matched
        .map(|m| {
            Ok::<_, LedgerError>(Match {
                selector: selector(&m.selector)?,
                values: m.values,
                anchors: m
                    .anchors
                    .iter()
                    .map(|id| recording(id))
                    .collect::<Result<_, LedgerError>>()?,
            })
        })
        .transpose()?;
    Ok(Decision {
        recording: recording(&raw.recording)?,
        selected: raw.selected,
        rejected,
        matched,
    })
}
fn quote(raw: model::Quote) -> Result<Quote, LedgerError> {
    let source = source(raw.source)?;
    let certainty = match raw.certainty.as_str() {
        "confirmed" => Certainty::Confirmed,
        "unverified" => Certainty::Unverified,
        _ => return Err(invalid()),
    };
    Ok(Quote {
        text: raw.text,
        certainty,
        selector: selector(&raw.selector)?,
        source: QuoteSource {
            recording: source.recording,
            note: source.note,
            digest: source.digest,
            at_secs: raw.at_secs,
        },
    })
}
fn source(raw: model::Source) -> Result<SourceDigest, LedgerError> {
    Ok(SourceDigest {
        recording: recording(&raw.recording)?,
        note: raw.note.into(),
        digest: Sha256Digest::from_hex(&raw.digest).ok_or_else(invalid)?,
    })
}
fn recording(value: &str) -> Result<RecordingId, LedgerError> {
    RecordingId::parse(value).map_err(|_| invalid())
}
fn selector(value: &str) -> Result<Selector, LedgerError> {
    match value {
        "participant" => Ok(Selector::Participant),
        "term" => Ok(Selector::Term),
        "tag" => Ok(Selector::Tag),
        "recent" => Ok(Selector::Recent),
        _ => Err(invalid()),
    }
}
