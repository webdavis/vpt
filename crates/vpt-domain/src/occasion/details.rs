use super::time::OccasionTime;

#[derive(Clone, PartialEq, Eq)]
pub struct Participant {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OccasionDetails {
    pub at: OccasionTime,
    pub duration_secs: Option<u64>,
    pub title: String,
    pub participants: Vec<Participant>,
    pub tags: Vec<String>,
}

impl OccasionDetails {
    pub fn is_valid(&self) -> bool {
        let mut bytes = self.at.as_str().len();
        let mut text = |value: &str, empty: bool| {
            bytes = bytes.saturating_add(value.len());
            (empty || !value.trim().is_empty())
                && value.len() <= 4 * 65_536
                && value.chars().count() <= 65_536
                && !value.contains('\0')
        };
        self.duration_secs
            .is_none_or(|value| value <= i64::MAX as u64)
            && self.participants.len() <= 256
            && self.tags.len() <= 256
            && text(&self.title, false)
            && self.participants.iter().all(|p| {
                (!p.name.trim().is_empty()
                    || p.email.as_ref().is_some_and(|s| !s.trim().is_empty()))
                    && text(&p.name, true)
                    && p.email.as_ref().is_none_or(|s| text(s, false))
            })
            && self.tags.iter().all(|tag| text(tag, false))
            && bytes <= 1024 * 1024
    }
}

impl std::fmt::Debug for OccasionDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccasionDetails").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Participant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Participant").finish_non_exhaustive()
    }
}
