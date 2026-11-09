use super::*;
use serde::Deserialize;
use vpt_application::ports::ReleaseVersion;
#[derive(Deserialize)]
struct Binding<'a> {
    schema: &'a str,
    source_artifact_sha256: &'a str,
    released_content_sha256: &'a str,
}
pub(super) fn check(text: &str, expected: ReleaseVersion) -> Result<(), LedgerError> {
    if text.len() > validate::MAX_BYTES {
        return Err(invalid());
    }
    let binding: Binding<'_> = serde_json::from_str(text).map_err(|_| invalid())?;
    if binding.schema != "vpt.redaction-report/1"
        || binding.source_artifact_sha256 != expected.source.hex()
        || binding.released_content_sha256 != expected.content.hex()
    {
        return Err(invalid());
    }
    Ok(())
}
