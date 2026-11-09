use super::{MAX_TEXT, ReleaseFileError, ReleaseFiles};
use vpt_application::ports::{ReleasePayload, ReleaseVersion, Stores};
mod verify;

impl ReleaseFiles {
    /// Publish or recover the exact ledger payload; completion is committed by the caller afterward.
    pub fn publish(&self, payload: &ReleasePayload) -> Result<ReleaseVersion, ReleaseFileError> {
        let row = &payload.release;
        if payload.text.len() > MAX_TEXT || payload.report.len() > MAX_TEXT {
            return Err(ReleaseFileError::Capacity);
        }
        let expected_report = self.drafts.join(format!(
            "{}.{}.{}.report.json",
            row.recording,
            row.stage.as_str(),
            row.sequence
        ));
        if row.sequence <= 0
            || row.finalized_at.is_some()
            || row.destination.parent() != Some(self.destination.as_path())
            || row.report_path != expected_report
            || self.stores.digest_bytes(payload.text.as_bytes()) != row.version.content
            || self.stores.digest_bytes(payload.report.as_bytes()) != row.version.report
        {
            return Err(ReleaseFileError::Invalid);
        }
        let files = [
            (
                &row.report_path,
                payload.report.as_bytes(),
                row.version.report,
                true,
            ),
            (
                &row.destination,
                payload.text.as_bytes(),
                row.version.content,
                false,
            ),
        ];
        // Check both destinations before creating either missing half.
        for (path, bytes, digest, private) in files {
            verify::check(&self.stores, path, bytes.len(), digest, private, false)?;
        }
        for (path, bytes, digest, private) in files {
            if !verify::check(&self.stores, path, bytes.len(), digest, private, false)? {
                self.stores.publish(path, None, bytes)?;
            }
        }
        for (path, bytes, digest, private) in files {
            if !verify::check(&self.stores, path, bytes.len(), digest, private, true)? {
                return Err(vpt_application::ports::StoreError::Changed(path.clone()).into());
            }
        }
        Ok(row.version)
    }
}
