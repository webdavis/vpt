use serde_json::{Value, json};
pub use vpt_domain::copy::{Document, Error, Identity, Kind, Provenance};
pub fn fields(document: &Document<'_>) -> Result<Value, Error> {
    let content = vpt_domain::copy::content(document)?;
    let mut value = match &document.identity {
        Identity::Recording(id) => json!({"recording":id.as_str()}),
        Identity::Occasion { id, sources } => {
            json!({"occasion":id.as_str(), "recordings":sources.iter().map(|id|id.as_str()).collect::<Vec<_>>()})
        }
    };
    let p = &document.provenance;
    let mut provenance = json!({"produced_by":"vpt", "engines":p.engines, "open_flags":p.open_flags, "reviewed":p.reviewed, "redacted":document.kind == Kind::Released});
    if let Some((at, offset)) = p.captured {
        provenance["captured_at"] = json!(at.rfc3339_with(offset));
    }
    if let Some(at) = p.occasion_at {
        provenance["occasion_at"] = json!(at.as_str());
    }
    value["note"] = json!(document.note);
    value["content_sha256"] = json!(document.digest.hex());
    value["title"] = json!(document.title());
    value["content"] = json!(content);
    value["provenance"] = provenance;
    value["generated_at"] = json!(document.generated_at.rfc3339());
    crate::bounded::encode(&value).map_err(|e| match e {
        crate::bounded::EncodeError::Invalid => Error::Invalid,
        crate::bounded::EncodeError::CapacityExceeded => Error::CapacityExceeded,
    })?;
    Ok(value)
}

#[cfg(test)]
mod tests;
