pub(super) mod inventory;
use std::collections::BTreeMap;
use vpt_application::ports::*;
use vpt_domain::review::{FlagContent, Resolution};

pub(super) fn artifacts_by_recording(
    artifacts: Vec<RetentionArtifact>,
) -> BTreeMap<String, Vec<RetentionArtifact>> {
    let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for artifact in artifacts {
        if let Some(id) = artifact.owner.recording() {
            grouped.entry(id.to_string()).or_default().push(artifact);
        }
    }
    grouped
}

pub(super) fn assemble(
    record: RecordingRecord,
    review: ReviewSnapshot,
    artifacts: Vec<RetentionArtifact>,
) -> RecordingObservation {
    let transcription = review.accepted.map(|accepted| {
        let mut engines = vec![accepted.transcript.engine];
        engines.extend(accepted.checker_engine);
        TranscriptionMetadata {
            engines,
            language: accepted.transcript.language,
            requested_language: accepted.requested_language,
            language_source: accepted.transcript.language_source,
            local: accepted.transcript.local,
            from_checker: accepted.from_checker,
        }
    });
    let mut flags = Vec::new();
    let mut diagnostics = Vec::new();
    for flag in review.flags {
        if flag.resolution != Resolution::Open {
            continue;
        }
        let (class, artifact, diagnostic) = match flag.content {
            FlagContent::Lexical(flag) if flag.class.surfaced() => {
                (flag.class.as_str(), None, false)
            }
            FlagContent::Lexical(_) => continue,
            FlagContent::Verification(finding) => {
                (finding.finding.as_str(), Some(finding.artifact), false)
            }
            FlagContent::Diagnostic(diagnostic) => (diagnostic.as_str(), None, true),
        };
        let observed = ObservedFlag {
            identifier: flag.identifier,
            class,
            artifact,
        };
        if diagnostic {
            diagnostics.push(observed);
        } else {
            flags.push(observed);
        }
    }
    flags.sort_by_key(|flag| flag.identifier.0);
    diagnostics.sort_by_key(|flag| flag.identifier.0);
    RecordingObservation {
        record,
        transcription,
        flags,
        diagnostics,
        stale_artifacts: review.stale_artifacts,
        artifacts,
    }
}
