use super::*;
use std::collections::BTreeSet;

pub(in crate::ledger::proposal::codec) fn check(
    proposal: &CheckedProposal,
) -> Result<(), LedgerError> {
    let mut items = 0usize;
    let mut token_bytes = 0usize;
    for checked in proposal.summary.iter().chain(&proposal.actions) {
        let grounded = &checked.grounding;
        if let Some(Finding::Unsupported(token)) = &grounded.finding {
            token_bytes = token_bytes.checked_add(token.len()).ok_or_else(invalid)?;
            if token_bytes > MAX_BODY {
                return Err(invalid());
            }
        }
        items = items
            .checked_add(grounded.sources.len())
            .and_then(|v| v.checked_add(grounded.inherited_flags.len()))
            .ok_or_else(invalid)?;
        if items > 65_536 || grounded.sources.len() > 256 {
            return Err(invalid());
        }
        let mut ids = BTreeSet::new();
        if grounded.inherited_flags.iter().any(|id| !ids.insert(id.0)) {
            return Err(invalid());
        }
        if grounded.sources.iter().any(|source| {
            source.start_secs >= source.end_secs || source.end_secs > 9_007_199_254_740_992
        }) {
            return Err(invalid());
        }
        let unresolved = matches!(
            grounded.finding,
            Some(Finding::Unsourced | Finding::BadReference)
        );
        if unresolved {
            if !grounded.sources.is_empty() || !grounded.inherited_flags.is_empty() {
                return Err(invalid());
            }
        } else if grounded.sources.len() != checked.claim.source_references.len()
            || grounded.sources.is_empty()
            || grounded
                .sources
                .iter()
                .zip(&checked.claim.source_references)
                .any(|(source, reference)| SourceRange::parse(reference) != Some(*source))
        {
            return Err(invalid());
        }
        match &grounded.finding {
            Some(Finding::Unsourced) if !checked.claim.source_references.is_empty() => {
                return Err(invalid());
            }
            Some(Finding::BadReference) if checked.claim.source_references.is_empty() => {
                return Err(invalid());
            }
            Some(Finding::BuiltOnFlaggedText) if grounded.inherited_flags.is_empty() => {
                return Err(invalid());
            }
            None if !grounded.inherited_flags.is_empty() => return Err(invalid()),
            Some(Finding::Unsupported(token))
                if token.trim().is_empty()
                    || token.len() > 262_144
                    || token.chars().count() > 65_536 =>
            {
                return Err(invalid());
            }
            _ => (),
        }
    }
    Ok(())
}
