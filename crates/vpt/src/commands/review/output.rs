use crate::cli::output::Outcome;
use serde_json::{Value, json};
use vpt_application::review::{FlagView, Review};

pub(super) fn success(review: &Review) -> Outcome {
    let flags: Vec<_> = review.flags.iter().map(flag_json).collect();
    let findings: Vec<_> = review
        .findings
        .iter()
        .map(|(id, finding)| finding_json(id, finding))
        .collect();
    let diagnostics: Vec<_> = review
        .diagnostics
        .iter()
        .map(|(id, diagnostic)| {
            json!({
                "identifier": id.hex(), "class": diagnostic.as_str(),
            })
        })
        .collect();
    let mut human = format!(
        "Review {}: {} open lexical flags, {} diagnostics\n",
        review.recording,
        flags.len(),
        diagnostics.len()
    );
    if !findings.is_empty() {
        human.push_str(&format!("{} open verification findings\n", findings.len()));
    }
    if !review.transcribed {
        human.push_str("No accepted transcript.\n")
    }
    for flag in &review.flags {
        let time = timecode(flag.occurrences[0].start);
        human.push_str(&format!(
            "[{time}] {} ({} occurrences) {}\n  {}",
            flag.flag.class.as_str(),
            flag.occurrences.len(),
            flag.identifier.hex(),
            json!(flag.flag.record_text)
        ));
        if let Some(alternative) = &flag.flag.alternative_text {
            human.push_str(&format!(" | {}", json!(alternative)))
        }
        human.push('\n');
    }
    for (id, finding) in &review.findings {
        human.push_str(&format!(
            "{} claim {}: {} {}\n  {}\n",
            finding.artifact.as_str(),
            finding.claim_index,
            finding.finding.as_str(),
            id.hex(),
            json!(finding.record_text)
        ));
    }
    for (id, diagnostic) in &review.diagnostics {
        human.push_str(&format!(
            "Diagnostic: {} {}\n",
            diagnostic.as_str(),
            id.hex()
        ))
    }
    Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        human,
        document: vpt_protocol::result::document(
            "review",
            json!({
                "id":review.recording.as_str(), "transcribed":review.transcribed,
                "open_flags":flags.len()+findings.len(), "flags":flags, "findings":findings, "diagnostic_count":diagnostics.len(), "diagnostics":diagnostics,
            }),
        ),
    }
}

fn flag_json(view: &FlagView) -> Value {
    let occurrences: Vec<_> = view
        .occurrences
        .iter()
        .map(|occurrence| {
            json!({
                "word_range":{"start":occurrence.words.start, "end":occurrence.words.end},
                "start":occurrence.start, "end":occurrence.end,
            })
        })
        .collect();
    json!({
        "identifier":view.identifier.hex(), "class":view.flag.class.as_str(),
        "record_text":view.flag.record_text, "alternative_text":view.flag.alternative_text,
        "confidence":view.flag.confidence, "occurrence_count":occurrences.len(),
        "timecode":timecode(view.occurrences[0].start), "occurrences":occurrences,
    })
}

fn timecode(seconds: f64) -> String {
    let seconds = seconds.floor() as u64;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn finding_json(
    id: &vpt_domain::digest::Sha256Digest,
    finding: &vpt_domain::review::VerificationFlag,
) -> Value {
    let sources = if finding.sources.is_empty() {
        Value::Null
    } else {
        json!(
            finding
                .sources
                .iter()
                .map(|range| json!({"start":range.start_secs,"end":range.end_secs}))
                .collect::<Vec<_>>()
        )
    };
    let token = match &finding.finding {
        vpt_domain::grounding::Finding::Unsupported(token) => Some(token.as_str()),
        _ => None,
    };
    json!({"identifier":id.hex(),"class":finding.finding.as_str(),"artifact":finding.artifact.as_str(),"claim_index":finding.claim_index,"claim_digest":finding.claim_digest.hex(),"record_text":finding.record_text,"unsupported_token":token,"sources":sources})
}

#[cfg(test)]
mod tests;
