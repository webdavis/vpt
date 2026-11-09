use super::{RenderError, writer::Writer};
use crate::{
    grounding::Finding,
    proposal::{CheckedClaim, CheckedProposal},
};
mod validate;

pub const FINDING_CORRECTION_MARKER: &str = "<!-- vpt:verification-correction -->";

pub fn analysis_content(proposal: &CheckedProposal) -> Result<String, RenderError> {
    validate::input(proposal)?;
    let mut output = Writer::default();
    section(&mut output, "Summary", &proposal.summary)?;
    output.literal("\n")?;
    section(&mut output, "Actions", &proposal.actions)?;
    Ok(output.finish())
}

fn section(output: &mut Writer, title: &str, claims: &[CheckedClaim]) -> Result<(), RenderError> {
    output.literal(&format!("## {title}\n"))?;
    if !claims.is_empty() {
        output.literal("\n")?;
    }
    for checked in claims {
        output.literal("- ")?;
        output.text(&checked.claim.text)?;
        for source in &checked.claim.source_references {
            output.literal(" [")?;
            if source
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b':' | b'-'))
            {
                output.literal(source)?;
            } else {
                output.text(source)?;
            }
            output.literal("]")?;
        }
        if let Some(finding) = &checked.grounding.finding {
            output.literal(" ")?;
            annotation(output, finding)?;
        }
        output.literal("\n")?;
    }
    Ok(())
}

pub fn finding_annotation(finding: &Finding) -> Result<String, RenderError> {
    if let Finding::Unsupported(token) = finding
        && (token.len() > 262_144 || token.chars().count() > 65_536)
    {
        return Err(RenderError::CapacityExceeded);
    }
    let mut output = Writer::default();
    annotation(&mut output, finding)?;
    Ok(output.finish())
}

pub fn resolved_finding_annotation(
    finding: &Finding,
    resolution: &crate::review::Resolution,
) -> Result<Option<String>, RenderError> {
    use crate::review::Resolution;
    match resolution {
        Resolution::Open => finding_annotation(finding).map(Some),
        Resolution::Confirmed { .. } | Resolution::Dismissed { .. } => Ok(None),
        Resolution::Corrected { text, .. } => {
            if !crate::review::valid_correction(text) {
                return Err(RenderError::InvalidFlags);
            }
            let mut output = Writer::default();
            output.literal("[corrected: ")?;
            output.text(text)?;
            output.literal("]")?;
            output.literal(FINDING_CORRECTION_MARKER)?;
            Ok(Some(output.finish()))
        }
    }
}

fn annotation(output: &mut Writer, finding: &Finding) -> Result<(), RenderError> {
    output.literal("[")?;
    match finding {
        Finding::Unsourced => output.literal("unsourced")?,
        Finding::BadReference => output.literal("bad-reference")?,
        Finding::BuiltOnFlaggedText => output.literal("built-on-flagged-text")?,
        Finding::Unsupported(token) => {
            output.literal("unsupported: ")?;
            output.text(token)?;
        }
    }
    output.literal("]")
}

#[cfg(test)]
mod tests;
