use super::engines::{transcribe, unspoken};
use super::plan::Run;
use crate::commands::transcribe::errors::cause;
use crate::report::Row;
use std::collections::BTreeSet;
use vpt_adapters::{Clip, converse};
use vpt_application::Settings;
use vpt_application::engines::{EngineConfig, EngineKind};
use vpt_application::ports::EngineFailure;

const DIALOGUE: [(&str, &str); 2] = [
    (
        "Samantha",
        "The orange lantern hummed quietly beside the silver canoe all evening.",
    ),
    (
        "Fred",
        "Seven paper owls waited patiently on the crooked garden wall.",
    ),
];

const REACH: &str =
    "; whisply sent the token to Hugging Face, and the speaker models download on first use";

pub(super) fn test(settings: &Settings) -> Result<Run<'_>, Row> {
    let engines = &settings.engines;
    let selected = engines.selected();
    let whisply = |engine: &&EngineConfig| matches!(engine.kind, EngineKind::Whisply { .. });
    let tokened = |engine: &&EngineConfig| {
        matches!(
            engine.kind,
            EngineKind::Whisply {
                hf_token: Some(_),
                ..
            }
        )
    };
    let all = || engines.definitions.values().filter(whisply);
    if let Some(engine) = all()
        .filter(tokened)
        .find(|engine| selected.contains(&engine.name))
    {
        return Ok(Box::new(move || rows(settings, engine)));
    }
    let detail = match all().find(tokened) {
        Some(engine) => format!(
            "the Hugging Face token is on `engines.{}`, which nothing selects; select it as \
             `engines.main` or `engines.checker` in the config",
            engine.name
        ),
        None => format!(
            "not configured, set `engines.{}.hf_token` in the config",
            all()
                .max_by_key(|engine| selected.contains(&engine.name))
                .map_or("whisply", |engine| engine.name.as_str())
        ),
    };
    Err(Row::not_run("token", detail))
}

fn rows(settings: &Settings, engine: &EngineConfig) -> Vec<Row> {
    let clip = match converse(&DIALOGUE, settings.source.max_audio_bytes) {
        Ok(clip) => clip,
        Err(error) => return vec![unspoken(error)],
    };
    let made = Row::ok(
        "clip",
        format!(
            "{:.1} s of two made-up English lines in two `say` voices, made in a private \
             temporary folder that is removed afterwards",
            clip.duration
        ),
    );
    vec![made, labels(settings, &clip, engine)]
}

fn labels(settings: &Settings, clip: &Clip, engine: &EngineConfig) -> Row {
    let (result, seconds) = transcribe(settings, clip, engine);
    let name = engine.name.as_str();
    let reach = if matches!(result, Err(EngineFailure::Unavailable)) {
        ""
    } else {
        REACH
    };
    let output = match result {
        Ok(output) => output,
        Err(failure) => {
            let advice = if matches!(failure, EngineFailure::SpeakerAnnotation(_)) {
                "with the Hugging Face account that owns the token, accept the terms of \
                 pyannote/speaker-diarization-3.1 and pyannote/segmentation-3.0, then run \
                 `vpt test speakers` again"
            } else {
                "see `vpt doctor speakers`, then whisply's own setup"
            };
            let detail = format!("failed in {seconds:.1} s: {}{reach}", cause(&failure));
            return Row::issue(name, detail).advice(advice);
        }
    };
    let found: BTreeSet<&str> = output
        .transcript
        .segments
        .iter()
        .filter_map(|segment| segment.speaker.as_deref())
        .collect();
    let plural = if found.len() == 1 { "" } else { "s" };
    let detail = format!(
        "{} speaker label{plural} in {seconds:.1} s{reach}",
        found.len()
    );
    if found.len() >= 2 {
        Row::ok(name, detail)
    } else {
        Row::issue(name, detail).advice(
            "whisply ran but did not tell the two voices apart; check that its speaker \
             annotation works on a recording of two people",
        )
    }
}
