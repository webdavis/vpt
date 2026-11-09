use super::*;
use vpt_domain::grounding::{Finding, GroundedClaim, SourceRange};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedGrounding {
    sources: Vec<[u64; 2]>,
    finding: Option<SavedFinding>,
    inherited_flags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "class",
    content = "token",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum SavedFinding {
    Unsourced,
    BadReference,
    Unsupported(String),
    BuiltOnFlaggedText,
}

pub(super) fn save(grounding: &GroundedClaim) -> SavedGrounding {
    SavedGrounding {
        sources: grounding
            .sources
            .iter()
            .map(|range| [range.start_secs, range.end_secs])
            .collect(),
        finding: grounding.finding.as_ref().map(|finding| match finding {
            Finding::Unsourced => SavedFinding::Unsourced,
            Finding::BadReference => SavedFinding::BadReference,
            Finding::Unsupported(token) => SavedFinding::Unsupported(token.clone()),
            Finding::BuiltOnFlaggedText => SavedFinding::BuiltOnFlaggedText,
        }),
        inherited_flags: grounding
            .inherited_flags
            .iter()
            .map(Sha256Digest::hex)
            .collect(),
    }
}

pub(super) fn restore(saved: SavedGrounding) -> Result<GroundedClaim, LedgerError> {
    Ok(GroundedClaim {
        sources: saved
            .sources
            .into_iter()
            .map(|[start_secs, end_secs]| SourceRange {
                start_secs,
                end_secs,
            })
            .collect(),
        finding: saved.finding.map(|finding| match finding {
            SavedFinding::Unsourced => Finding::Unsourced,
            SavedFinding::BadReference => Finding::BadReference,
            SavedFinding::Unsupported(token) => Finding::Unsupported(token),
            SavedFinding::BuiltOnFlaggedText => Finding::BuiltOnFlaggedText,
        }),
        inherited_flags: saved
            .inherited_flags
            .into_iter()
            .map(|id| digest(&id))
            .collect::<Result<_, _>>()?,
    })
}

mod validate;
pub(super) use validate::check;
