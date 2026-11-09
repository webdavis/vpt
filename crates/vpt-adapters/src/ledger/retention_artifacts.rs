pub(super) mod completion;
use std::path::Path;
use vpt_application::ports::*;
use vpt_domain::digest::Sha256Digest;

pub(super) const MAX_ARTIFACTS: usize = 65_536;
const MAX_METADATA: usize = 16 * 1024 * 1024;

pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid retention artifact metadata".into())
}

#[derive(Default)]
pub(super) struct Inventory {
    rows: Vec<RetentionArtifact>,
    bytes: usize,
}

impl Inventory {
    pub(super) fn push(
        &mut self,
        kind: ArtifactKind,
        owner: &str,
        path: &Path,
        digest: Sha256Digest,
        status: RetentionStatus,
        trashed_at: Option<vpt_domain::time::UtcInstant>,
    ) -> Result<(), LedgerError> {
        let text = vpt_domain::brief::document::source_path(path).ok_or_else(invalid)?;
        let bytes = text
            .len()
            .checked_add(std::mem::size_of::<RetentionArtifact>() + owner.len())
            .ok_or_else(invalid)?;
        if self.rows.len() >= MAX_ARTIFACTS || bytes > MAX_METADATA.saturating_sub(self.bytes) {
            return Err(LedgerError::Corrupt(
                "retention artifact capacity exceeded".into(),
            ));
        }
        let owner = ArtifactOwner::parse(kind, owner).ok_or_else(invalid)?;
        self.bytes += bytes;
        self.rows.push(RetentionArtifact {
            kind,
            owner,
            path: path.into(),
            digest,
            status,
            trashed_at,
        });
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<Vec<RetentionArtifact>, LedgerError> {
        self.rows.sort_by(|a, b| a.path.cmp(&b.path));
        if self
            .rows
            .windows(2)
            .any(|pair| pair[0].path == pair[1].path)
        {
            return Err(invalid());
        }
        Ok(self.rows)
    }
}

#[cfg(test)]
mod tests;
