use super::plan::Run;
use crate::commands::transcribe::errors::cause;
use crate::report::Row;
use std::time::Instant;
use vpt_adapters::{Clip, GuardedEngines, SpeechError, speak};
use vpt_application::Settings;
use vpt_application::engine_trial::{LANGUAGE, SENTENCE, heard};
use vpt_application::engines::EngineConfig;
use vpt_application::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};

pub(super) fn test(settings: &Settings) -> Result<Run<'_>, Row> {
    if settings.engines.selected().is_empty() {
        return Err(Row::not_run(
            "configured",
            "no engine is selected, set `engines.main` in the config",
        ));
    }
    Ok(Box::new(|| rows(settings)))
}

fn rows(settings: &Settings) -> Vec<Row> {
    let clip = match speak(SENTENCE, settings.source.max_audio_bytes) {
        Ok(clip) => clip,
        Err(error) => return vec![unspoken(error)],
    };
    let made = Row::ok(
        "clip",
        format!(
            "{:.1} s of made-up English speech, made with `say` in a private temporary folder that \
             is removed afterwards",
            clip.duration
        ),
    );
    let engines = &settings.engines;
    std::iter::once(made)
        .chain(
            engines
                .selected()
                .into_iter()
                .filter_map(|name| engines.definitions.get(name))
                .map(|config| row(settings, &clip, config)),
        )
        .collect()
}

pub(super) fn unspoken(error: SpeechError) -> Row {
    let (program, detail) = match error {
        SpeechError::Missing(program) => (
            program,
            format!("could not make the test clip: macOS `{program}` was not found"),
        ),
        SpeechError::Failed(program, reason) => (
            program,
            format!("could not make the test clip with `{program}`: {reason}"),
        ),
    };
    Row::issue("clip", detail).advice(format!(
        "vpt makes test clips with macOS `say` and `afconvert`; make sure `{program}` runs from \
         your PATH"
    ))
}

pub(super) fn transcribe(
    settings: &Settings,
    clip: &Clip,
    config: &EngineConfig,
) -> (Result<EngineOutput, EngineFailure>, f64) {
    let started = Instant::now();
    let result = GuardedEngines::new(
        settings.helper_path.clone(),
        clip.digest,
        settings.source.max_audio_bytes,
    )
    .transcribe(&EngineRequest {
        engine: config,
        audio: &clip.path,
        out_dir: clip.directory(),
        language: LANGUAGE,
        duration: clip.duration,
        timeout: settings.engines.timeout,
    });
    (result, started.elapsed().as_secs_f64())
}

fn row(settings: &Settings, clip: &Clip, config: &EngineConfig) -> Row {
    let (result, seconds) = transcribe(settings, clip, config);
    let sent = !config.local() && !matches!(result, Err(EngineFailure::Unavailable));
    let notes = format!(
        "{}{}",
        languages(settings, &config.name),
        if sent {
            "; the made-up clip left the machine"
        } else {
            ""
        }
    );
    let name = config.name.as_str();
    match result {
        Ok(output) => {
            let text: Vec<&str> = output
                .transcript
                .segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect();
            let heard = heard(&text.join(" "));
            let detail = format!(
                "heard {} of {} words in {seconds:.1} s{notes}",
                heard.found, heard.expected
            );
            if heard.enough() {
                Row::ok(name, detail)
            } else {
                Row::issue(name, detail).advice(
                    "the engine ran but missed most of the test sentence; check its model and \
                     language settings",
                )
            }
        }
        Err(failure) => Row::issue(
            name,
            format!("failed in {seconds:.1} s: {}{notes}", cause(&failure)),
        )
        .advice("see `vpt doctor engines`, then the engine's own setup"),
    }
}

fn languages(settings: &Settings, name: &str) -> String {
    let uses = |pair: &vpt_application::engines::EnginePair| {
        pair.main == name || pair.checker.as_deref() == Some(name)
    };
    if uses(&settings.engines.pair) {
        return String::new();
    }
    let tags: Vec<&str> = settings
        .engines
        .by_language
        .iter()
        .filter(|(_, pair)| uses(pair))
        .map(|(tag, _)| tag.as_str())
        .collect();
    format!("; selected for {}, run on English speech", tags.join(", "))
}
