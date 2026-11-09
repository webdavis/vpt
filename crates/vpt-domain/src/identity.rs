use crate::digest::Sha256Digest;
use crate::time::{Civil, UtcInstant, UtcOffset};

const TIMESTAMP_LEN: usize = 17;
const IDENTITY_LEN: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordingId(String);

#[derive(Debug, PartialEq, Eq)]
pub enum IdentityError {
    Malformed,
    Unrepresentable,
}

pub fn local_timestamp(captured: UtcInstant, offset: UtcOffset) -> String {
    let civil = captured.civil(offset);
    format!(
        "{:04}-{:02}-{:02}T{:02}{:02}{:02}",
        civil.year, civil.month, civil.day, civil.hour, civil.minute, civil.second
    )
}

pub fn parse_local_timestamp(text: &str) -> Result<Civil, IdentityError> {
    let bytes = text.as_bytes();
    let shape_ok = bytes.len() == TIMESTAMP_LEN
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[..4]
            .iter()
            .chain(&bytes[5..7])
            .chain(&bytes[8..10])
            .chain(&bytes[11..])
            .all(u8::is_ascii_digit);
    if !shape_ok {
        return Err(IdentityError::Malformed);
    }
    let field = |range: std::ops::Range<usize>| {
        text[range]
            .parse::<u32>()
            .map_err(|_| IdentityError::Malformed)
    };
    let civil = Civil {
        year: i64::from(field(0..4)?),
        month: field(5..7)?,
        day: field(8..10)?,
        hour: field(11..13)?,
        minute: field(13..15)?,
        second: field(15..17)?,
    };
    if civil.is_valid() {
        Ok(civil)
    } else {
        Err(IdentityError::Malformed)
    }
}

impl RecordingId {
    pub fn derive(
        captured: UtcInstant,
        offset: UtcOffset,
        digest: &Sha256Digest,
    ) -> Result<Self, IdentityError> {
        Self::parse(&format!(
            "{}-{}",
            local_timestamp(captured, offset),
            digest.hash12()
        ))
        .map_err(|_| IdentityError::Unrepresentable)
    }

    pub fn parse(text: &str) -> Result<Self, IdentityError> {
        let bytes = text.as_bytes();
        let shape_ok = bytes.len() == IDENTITY_LEN
            && bytes[TIMESTAMP_LEN] == b'-'
            && bytes[TIMESTAMP_LEN + 1..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte));
        if !shape_ok {
            return Err(IdentityError::Malformed);
        }
        parse_local_timestamp(&text[..TIMESTAMP_LEN])?;
        Ok(Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn local_timestamp(&self) -> &str {
        &self.0[..TIMESTAMP_LEN]
    }

    pub fn archived_offset(&self, captured: UtcInstant) -> Option<UtcOffset> {
        let local = parse_local_timestamp(self.local_timestamp()).ok()?;
        let as_utc = local.instant(UtcOffset { secs: 0 })?;
        let difference = i128::from(as_utc.secs) - i128::from(captured.secs);
        let secs = i32::try_from(difference).ok()?;
        (secs.unsigned_abs() < 86_400).then_some(UtcOffset { secs })
    }

    pub fn capture_date(&self) -> &str {
        &self.0[..10]
    }

    pub fn hash12(&self) -> &str {
        &self.0[TIMESTAMP_LEN + 1..]
    }
}

impl std::fmt::Display for RecordingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
