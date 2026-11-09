use crate::{markdown_claims, verification_flag_id};
use sha2::{Digest, Sha256};
use vpt_application::ports::{AcceptedTranscript, NoteStage};
use vpt_domain::{
    digest::Sha256Digest,
    grounding::{Finding, Grounder, SourceRange},
    review::{Resolution, ReviewFlag, VerificationFlag},
};

pub struct FindingLine {
    pub finding: Finding,
    pub line: usize,
    pub sources: Vec<SourceRange>,
}

pub struct Checked {
    pub report: Vec<FindingLine>,
    pub findings: Vec<VerificationFlag>,
    pub content: String,
    pub input_digest: Sha256Digest,
    pub open: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckError;

pub fn check(
    content: &str,
    artifact: Option<NoteStage>,
    transcript: &AcceptedTranscript,
    flags: &[ReviewFlag],
    window: u32,
) -> Result<Checked, CheckError> {
    if flags.len() > 65_539 {
        return Err(CheckError);
    }
    let parsed = if let Some(stage) = artifact {
        markdown_claims::extract_owned(content, stage, flags)
    } else {
        markdown_claims::extract(content)
    }
    .map_err(|_| CheckError)?;
    let mut grounder = Grounder::new(
        transcript.transcript.segments.iter().flat_map(|s| &s.words),
        flags,
        transcript.duration,
        window,
    )
    .map_err(|_| CheckError)?;
    let mut checked = Checked {
        report: vec![],
        findings: vec![],
        content: String::new(),
        input_digest: Sha256Digest(Sha256::digest(content.as_bytes()).into()),
        open: 0,
    };
    let mut annotations = Vec::new();
    let mut resolutions = Vec::new();
    let previous: std::collections::HashMap<_, _> = flags
        .iter()
        .map(|flag| (flag.identifier, &flag.resolution))
        .collect();
    for claim in parsed.claims() {
        let grounded = grounder.check(&claim.claim).map_err(|_| CheckError)?;
        let mut resolution = Resolution::Open;
        if let Some(finding) = &grounded.finding {
            checked.report.push(FindingLine {
                finding: finding.clone(),
                line: claim.line(),
                sources: grounded.sources.clone(),
            });
            if let Some(stage) = artifact {
                let flag = VerificationFlag {
                    artifact: stage,
                    claim_index: claim.index,
                    claim_digest: Sha256Digest(
                        Sha256::digest(claim.identity_text.as_bytes()).into(),
                    ),
                    record_text: claim.identity_text.clone(),
                    finding: finding.clone(),
                    sources: grounded.sources,
                };
                let id = verification_flag_id(&transcript.recording, &flag);
                resolution = previous
                    .get(&id)
                    .map_or(Resolution::Open, |old| (*old).clone());
                checked.open += usize::from(resolution == Resolution::Open);
                checked.findings.push(flag);
            }
        }
        annotations.push(grounded.finding);
        resolutions.push(resolution);
    }
    if artifact.is_some() {
        checked.content = parsed
            .annotate_resolved(&annotations, &resolutions)
            .map_err(|_| CheckError)?;
    }
    Ok(checked)
}

pub fn review_flags(
    transcript: &AcceptedTranscript,
    artifact: NoteStage,
    findings: &[VerificationFlag],
    previous: &[ReviewFlag],
) -> Result<Vec<ReviewFlag>, CheckError> {
    crate::ledger::prepare_verification_findings(
        &transcript.recording,
        artifact,
        findings,
        transcript,
        previous,
    )
    .map_err(|_| CheckError)
}
