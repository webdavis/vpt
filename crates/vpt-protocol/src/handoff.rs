use serde_json::{Value, json};
pub use vpt_domain::handoff::{Document, Error, Identity, Kind, Provenance};
pub const SCHEMA: &str = "vpt.handoff/1";
pub fn value(document: &Document<'_>) -> Result<Value, Error> {
    let content = vpt_domain::handoff::content(document)?;
    let mut identity = match &document.identity {
        Identity::Recording(id) => json!({"recording":id.as_str()}),
        Identity::Occasion { id, sources } => {
            json!({"occasion":id.as_str(), "recordings":sources.iter().map(|id|id.as_str()).collect::<Vec<_>>()})
        }
    };
    identity["note"] = json!(document.note);
    identity["content_sha256"] = json!(document.digest.hex());
    let p = &document.provenance;
    let mut provenance = json!({"produced_by":"vpt", "engines":p.engines, "open_flags":p.open_flags, "reviewed":p.reviewed, "redacted":document.kind == Kind::Released});
    if let Some((at, offset)) = p.captured {
        provenance["captured_at"] = json!(at.rfc3339_with(offset));
    }
    if let Some(at) = p.occasion_at {
        provenance["occasion_at"] = json!(at.as_str());
    }
    let value = json!({"schema":SCHEMA,"kind":document.kind.as_str(),"identity":identity,"title":document.title(),"content":content,"provenance":provenance,"generated_at":document.generated_at.rfc3339()});
    crate::bounded::encode(&value).map_err(|e| match e {
        crate::bounded::EncodeError::Invalid => Error::Invalid,
        crate::bounded::EncodeError::CapacityExceeded => Error::CapacityExceeded,
    })?;
    Ok(value)
}

#[cfg(test)]
mod tests;
