use super::plan::Run;
use crate::commands::analyze::errors::command;
use crate::report::Row;
use std::time::Instant;
use vpt_adapters::{CommandAgent, SystemClock, private_tempdir};
use vpt_application::Settings;
use vpt_application::ports::{AgentCommand, AnalysisFailure, AnalysisRequest, Clock};
use vpt_application::tag_hints::TagHints;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId};

const TRANSCRIPT: &str = "[00:00] Speaker 1: This is a made-up meeting that vpt test sends to \
                          check the summary command.\n\n[00:06] Speaker 2: The purple kettle club \
                          agreed to paint the paper lighthouse yellow.\n\n[00:12] Speaker 1: I \
                          will order the yellow paint by Friday.\n";

const REACH: &str = "; the made-up transcript went to whatever the command calls";

pub(super) fn test(settings: &Settings) -> Result<Run<'_>, Row> {
    if settings.analysis.command.is_empty() {
        return Err(Row::not_run(
            "command",
            "not configured, set `analysis.command` in the config",
        ));
    }
    Ok(Box::new(|| vec![row(settings)]))
}

fn row(settings: &Settings) -> Row {
    let started = Instant::now();
    let result = analyze(settings);
    let seconds = started.elapsed().as_secs_f64();
    match result {
        Ok(by) => Row::ok(
            "command",
            format!("a valid vpt.proposal/1 from {by} in {seconds:.1} s{REACH}"),
        ),
        Err(failure) => {
            let reach = if matches!(failure, AnalysisFailure::Unavailable) {
                ""
            } else {
                REACH
            };
            let advice = match failure {
                AnalysisFailure::InvalidDocument | AnalysisFailure::UnsupportedVersion(_) => {
                    "the command must print exactly one vpt.proposal/1 JSON document and nothing \
                     else; see \"Analysis harnesses\" in the README"
                }
                _ => {
                    "see `vpt doctor summary`, then run the command by hand with a short \
                     transcript on standard input"
                }
            };
            let reason = match failure {
                AnalysisFailure::Exited(code) => format!("analysis command exited {code}"),
                AnalysisFailure::Signaled(signal) => {
                    format!("analysis command ended by signal {signal}")
                }
                failure => command(failure).message,
            };
            Row::issue(
                "command",
                format!("failed in {seconds:.1} s: {reason}{reach}"),
            )
            .advice(advice)
        }
    }
}

fn analyze(settings: &Settings) -> Result<String, AnalysisFailure> {
    let folder = private_tempdir().map_err(|_| AnalysisFailure::Io)?;
    let transcript = folder.path().join("made-up-transcript.md");
    std::fs::write(&transcript, TRANSCRIPT).map_err(|_| AnalysisFailure::Io)?;
    let now = SystemClock.now();
    let offset = SystemClock.offset_at(now);
    let recording = RecordingId::derive(now, offset, &Sha256Digest([0; 32]))
        .map_err(|_| AnalysisFailure::InvalidArguments)?;
    let output = CommandAgent.analyze(&AnalysisRequest {
        settings: &settings.analysis,
        transcript: &transcript,
        recording: &recording,
        language: "en-US",
        captured: (now, offset),
        content: TRANSCRIPT,
        tags: &TagHints::default(),
    })?;
    Ok(output.proposal.by)
}
