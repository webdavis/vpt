use crate::additive::collect;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn unknown_fields(value: &Value) -> Vec<String> {
    let mut fields = BTreeSet::new();
    collect(
        value,
        "",
        &[
            "id",
            "created",
            "input_filepath",
            "output_filepath",
            "written_files",
            "device",
            "model",
            "transcription",
        ],
        &mut fields,
    );
    if let Some(transcriptions) = value["transcription"].as_object() {
        for transcription in transcriptions.values() {
            collect(
                transcription,
                "transcription.",
                &["text", "chunks", "text_with_speaker_annotation"],
                &mut fields,
            );
            if let Some(chunks) = transcription["chunks"].as_array() {
                for chunk in chunks {
                    collect(
                        chunk,
                        "transcription.chunks.",
                        &["text", "timestamp", "words", "speaker"],
                        &mut fields,
                    );
                    if let Some(words) = chunk["words"].as_array() {
                        for word in words {
                            collect(
                                word,
                                "transcription.chunks.words.",
                                &["word", "start", "end", "score", "speaker"],
                                &mut fields,
                            );
                        }
                    }
                }
            }
        }
    }
    fields
        .into_iter()
        .map(|field| format!("unknown additive whisply field: {field}"))
        .collect()
}
