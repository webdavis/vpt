use crate::ports::{LedgerError, ReviewLedger};
use std::ops::Range;
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    uncertainty::{Diagnostic, LexicalFlag},
};

pub mod alerts;
mod model;
pub mod resolve;
mod timing;
pub use model::{FlagView, Occurrence, Review, ReviewError};

pub fn review(ledger: &impl ReviewLedger, id: &RecordingId) -> Result<Review, ReviewError> {
    use vpt_domain::review::{FlagContent, Resolution};
    let snapshot = ledger
        .review_snapshot(id)
        .map_err(ReviewError::Ledger)?
        .ok_or(ReviewError::NotFound)?;
    let mut report = Review {
        recording: id.clone(),
        transcribed: snapshot.accepted.is_some(),
        flags: vec![],
        findings: vec![],
        diagnostics: vec![],
    };
    let Some(accepted) = snapshot.accepted else {
        return if snapshot.flags.is_empty() {
            Ok(report)
        } else {
            Err(ReviewError::InvalidFlags)
        };
    };
    if accepted.recording != *id || snapshot.flags.len() > 65_539 {
        return Err(ReviewError::InvalidFlags);
    }
    let words: Vec<_> = accepted
        .transcript
        .segments
        .iter()
        .flat_map(|segment| &segment.words)
        .take(65_537)
        .collect();
    if words.len() > 65_536 {
        return Err(ReviewError::InvalidFlags);
    }
    let mut count = 0usize;
    for flag in snapshot.flags {
        if flag.resolution != Resolution::Open {
            continue;
        }
        match flag.content {
            FlagContent::Diagnostic(diagnostic) => {
                report.diagnostics.push((flag.identifier, diagnostic))
            }
            FlagContent::Lexical(lexical) if lexical.class.surfaced() => {
                count = count.saturating_add(lexical.ranges.len());
                if count > 65_536 || lexical.ranges.is_empty() {
                    return Err(ReviewError::InvalidFlags);
                }
                let occurrences = lexical
                    .ranges
                    .iter()
                    .map(|range| timing::occurrence(range, &words, accepted.duration))
                    .collect::<Result<_, _>>()?;
                report.flags.push(FlagView {
                    identifier: flag.identifier,
                    flag: lexical,
                    occurrences,
                });
            }
            FlagContent::Lexical(_) => {}
            FlagContent::Verification(finding) => {
                if !finding.valid() {
                    return Err(ReviewError::InvalidFlags);
                }
                report.findings.push((flag.identifier, finding));
            }
        }
    }
    report
        .findings
        .sort_by_key(|(id, finding)| (finding.artifact, finding.claim_index, id.0));
    report.flags.sort_by_key(|view| {
        (
            view.flag.class,
            view.flag.ranges[0].start,
            view.identifier.0,
        )
    });
    report
        .diagnostics
        .sort_by_key(|(_, diagnostic)| diagnostic.as_str());
    Ok(report)
}

#[cfg(test)]
mod tests;
