use super::time::OccasionTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OccasionSource {
    Manual,
    Dam,
    Google,
}

impl OccasionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Dam => "dam",
            Self::Google => "google",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OccasionKey {
    source: OccasionSource,
    parts: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyError {
    Invalid,
    CapacityExceeded,
}

impl OccasionKey {
    pub fn manual(at: &OccasionTime, title: &str) -> Result<Self, KeyError> {
        let title = crate::note::filename::sanitized_title(title, usize::MAX)
            .map_err(|_| KeyError::CapacityExceeded)?;
        Self::new(OccasionSource::Manual, &[at.as_str(), &title])
    }
    pub fn dam(object: &str) -> Result<Self, KeyError> {
        Self::new(OccasionSource::Dam, &[object])
    }
    pub fn google(calendar: &str, event: &str) -> Result<Self, KeyError> {
        Self::new(OccasionSource::Google, &[calendar, event])
    }
    fn new(source: OccasionSource, parts: &[&str]) -> Result<Self, KeyError> {
        for part in parts {
            if part.is_empty() || part.chars().any(char::is_control) {
                return Err(KeyError::Invalid);
            }
            if part.len() > 4 * 65_536 || part.chars().count() > 65_536 {
                return Err(KeyError::CapacityExceeded);
            }
        }
        Ok(Self {
            source,
            parts: parts.iter().map(|s| (*s).into()).collect(),
        })
    }
    pub fn parse(text: &str) -> Result<Self, KeyError> {
        if text.len() > 8 * 65_536 + 64 {
            return Err(KeyError::CapacityExceeded);
        }
        let (source, mut tail) = text.split_once(':').ok_or(KeyError::Invalid)?;
        let mut parts = Vec::new();
        while !tail.is_empty() && parts.len() < 2 {
            let (size, remaining) = tail.split_once(':').ok_or(KeyError::Invalid)?;
            let size: usize = size.parse().map_err(|_| KeyError::Invalid)?;
            parts.push(remaining.get(..size).ok_or(KeyError::Invalid)?);
            tail = remaining.get(size..).ok_or(KeyError::Invalid)?;
        }
        let key = match (source, parts.as_slice()) {
            ("dam", [object]) => Self::dam(object)?,
            ("google", [calendar, event]) => Self::google(calendar, event)?,
            ("manual", [at, title]) => Self::manual(
                &OccasionTime::parse(at).map_err(|_| KeyError::Invalid)?,
                title,
            )?,
            _ => return Err(KeyError::Invalid),
        };
        if !tail.is_empty() || key.encoded() != text {
            return Err(KeyError::Invalid);
        }
        Ok(key)
    }
    pub fn encoded(&self) -> String {
        let mut result = format!("{}:", self.source.as_str());
        for part in &self.parts {
            result.push_str(&format!("{}:{part}", part.len()));
        }
        result
    }
    pub fn source(&self) -> OccasionSource {
        self.source
    }
}
impl std::fmt::Debug for OccasionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccasionKey")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}
#[cfg(test)]
mod tests;
