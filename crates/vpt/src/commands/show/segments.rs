use super::errors;
use serde_json::{Value, json};
use vpt_application::ports::ReviewSnapshot;
use vpt_protocol::error::ErrorDocument;

pub(super) fn json(review: &ReviewSnapshot) -> Result<Value, Box<ErrorDocument>> {
    let accepted = review.accepted.as_ref().ok_or_else(errors::missing)?;
    let texts = vpt_domain::note::corrected_segments(&accepted.transcript, &review.flags)
        .map_err(|_| errors::document(vpt_domain::copy::Error::Invalid))?;
    Ok(accepted
        .transcript
        .segments
        .iter()
        .zip(texts)
        .map(|(segment, text)| {
            json!({
                "start": segment.start,
                "end": segment.end,
                "timecode": super::timecode(segment.start as u64),
                "text": text,
            })
        })
        .collect())
}
