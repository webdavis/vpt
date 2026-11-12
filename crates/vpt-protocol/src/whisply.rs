mod diagnostics;
mod model;
mod turns;
use crate::engine::{
    self, DecodedEngine, EngineDecodeError, EngineDocument, EngineMetadata, ExpectedEngine,
};

pub struct WhisplyRequest<'a> {
    pub name: &'a str,
    pub language: &'a str,
    pub backend_language: &'a str,
    pub device: &'a str,
    pub duration: f64,
}

pub fn decode(
    bytes: &[u8],
    request: &WhisplyRequest<'_>,
) -> Result<DecodedEngine, EngineDecodeError> {
    let value = crate::limits::read_whisply(bytes).map_err(EngineDecodeError::Limit)?;
    let mut diagnostics = diagnostics::unknown_fields(&value);
    let mut backend: model::Document = serde_json::from_value(value)
        .map_err(|_| EngineDecodeError::Invalid("whisply document"))?;
    let supported_device = match request.device {
        "auto" => matches!(backend.device.as_str(), "cpu" | "mlx" | "cuda:0"),
        "gpu" => matches!(backend.device.as_str(), "cpu" | "cuda:0"),
        "cpu" | "mlx" => backend.device == request.device,
        _ => false,
    };
    if !supported_device {
        return Err(EngineDecodeError::Invalid("whisply device"));
    }
    if request.device == "gpu" && backend.device == "cpu" {
        diagnostics.push("whisply reported CPU for the configured GPU device".into());
    }
    let transcription = backend
        .transcription
        .remove(request.backend_language)
        .ok_or(EngineDecodeError::Invalid("whisply language"))?;
    if transcription.text.trim().is_empty() {
        return Err(EngineDecodeError::Invalid("empty transcript"));
    }
    let document = EngineDocument {
        schema: "vpt.engine/1".into(),
        engine: EngineMetadata {
            name: request.name.into(),
            tool: "whisply".into(),
            version: "unreported".into(),
            model: backend.model,
            family: "whisper".into(),
        },
        language: request.language.into(),
        local: true,
        segments: transcription
            .chunks
            .into_iter()
            .flat_map(turns::segments)
            .collect(),
    };
    let expected = ExpectedEngine {
        name: request.name.into(),
        family: "whisper".into(),
        local: true,
    };
    engine::validate::validate(&document, &expected, request.duration)?;
    diagnostics.extend([
        "whisply language reflects the requested backend option, not independent detection".into(),
        "whisply did not report its tool version".into(),
    ]);
    Ok(DecodedEngine {
        document,
        diagnostics,
    })
}

#[cfg(test)]
mod tests;
