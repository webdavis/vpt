use vpt_domain::transcript::{self as domain, LanguageSource};
use vpt_protocol::engine as wire;

pub(crate) fn from_document(
    document: wire::EngineDocument,
    source: LanguageSource,
) -> domain::Transcript {
    domain::Transcript {
        engine: from_metadata(document.engine),
        language: document.language,
        language_source: source,
        local: document.local,
        segments: document
            .segments
            .into_iter()
            .map(|segment| domain::Segment {
                start: segment.start,
                end: segment.end,
                text: segment.text,
                words: segment
                    .words
                    .into_iter()
                    .map(|word| domain::Word {
                        text: word.text,
                        start: word.start,
                        end: word.end,
                        confidence: word.confidence,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub(crate) fn to_document(transcript: &domain::Transcript) -> wire::EngineDocument {
    wire::EngineDocument {
        schema: "vpt.engine/1".into(),
        engine: to_metadata(&transcript.engine),
        language: transcript.language.clone(),
        local: transcript.local,
        segments: transcript
            .segments
            .iter()
            .map(|segment| wire::Segment {
                start: segment.start,
                end: segment.end,
                text: segment.text.clone(),
                words: segment
                    .words
                    .iter()
                    .map(|word| wire::Word {
                        text: word.text.clone(),
                        start: word.start,
                        end: word.end,
                        confidence: word.confidence,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub(crate) fn from_metadata(engine: wire::EngineMetadata) -> domain::EngineMetadata {
    domain::EngineMetadata {
        name: engine.name,
        tool: engine.tool,
        version: engine.version,
        model: engine.model,
        family: engine.family,
    }
}
pub(crate) fn to_metadata(engine: &domain::EngineMetadata) -> wire::EngineMetadata {
    wire::EngineMetadata {
        name: engine.name.clone(),
        tool: engine.tool.clone(),
        version: engine.version.clone(),
        model: engine.model.clone(),
        family: engine.family.clone(),
    }
}
