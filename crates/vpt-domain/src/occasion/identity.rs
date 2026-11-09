use super::time::OccasionTime;
use crate::{digest::Sha256Digest, identity::IdentityError};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OccasionId(String);

impl OccasionId {
    pub fn derive(at: &OccasionTime, digest: &Sha256Digest) -> Self {
        Self(format!("{}-{}", at.local_timestamp(), digest.hash12()))
    }
    pub fn parse(text: &str) -> Result<Self, IdentityError> {
        let bytes = text.as_bytes();
        if bytes.len() != 30
            || !text.is_ascii()
            || bytes[17] != b'-'
            || !bytes[18..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(IdentityError::Malformed);
        }
        let timestamp = if &text[15..17] == "60" {
            format!("{}59", &text[..15])
        } else {
            text[..17].into()
        };
        crate::identity::parse_local_timestamp(&timestamp)?;
        Ok(Self(text.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn date(&self) -> &str {
        &self.0[..10]
    }
    pub fn hash12(&self) -> &str {
        &self.0[18..]
    }
}

impl std::fmt::Display for OccasionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
