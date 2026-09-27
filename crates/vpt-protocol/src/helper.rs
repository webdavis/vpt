use serde::Deserialize;

pub const HELPER_SCHEMA: &str = "vpt.helper/1";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HelperVersion {
    pub schema: String,
    pub version: String,
}

impl HelperVersion {
    pub fn major(&self) -> Option<u32> {
        let mut core = self.version.as_str();
        if let Some((prefix, build)) = core.split_once('+') {
            if !identifiers(build, false) {
                return None;
            }
            core = prefix;
        }
        if let Some((prefix, prerelease)) = core.split_once('-') {
            if !identifiers(prerelease, true) {
                return None;
            }
            core = prefix;
        }
        let parts: Vec<_> = core.split('.').collect();
        if parts.len() != 3
            || parts.iter().any(|part| {
                part.is_empty()
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || (part.len() > 1 && part.starts_with('0'))
            })
        {
            return None;
        }
        parts[0].parse().ok()
    }
}

fn identifiers(value: &str, prerelease: bool) -> bool {
    value.split('.').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !(prerelease
                && part.len() > 1
                && part.starts_with('0')
                && part.bytes().all(|byte| byte.is_ascii_digit()))
    })
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Trashed {
    pub trashed: String,
}

#[cfg(test)]
mod tests;
