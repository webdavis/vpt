use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    Confirmed,
    Corrected,
    Dismissed,
}
impl ResolutionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Corrected => "corrected",
            Self::Dismissed => "dismissed",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "confirmed" => Some(Self::Confirmed),
            "corrected" => Some(Self::Corrected),
            "dismissed" => Some(Self::Dismissed),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum ReviewAction {
    Confirm,
    Correct(String),
    Dismiss,
}
impl std::fmt::Debug for ReviewAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Confirm => "Confirm",
            Self::Correct(_) => "Correct",
            Self::Dismiss => "Dismiss",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
    HiddenFlag,
    DiagnosticCorrection,
    InvalidText,
}

impl Resolution {
    pub fn kind(&self) -> Option<ResolutionKind> {
        match self {
            Self::Open => None,
            Self::Confirmed { .. } => Some(ResolutionKind::Confirmed),
            Self::Corrected { .. } => Some(ResolutionKind::Corrected),
            Self::Dismissed { .. } => Some(ResolutionKind::Dismissed),
        }
    }
    pub fn at(&self) -> Option<UtcInstant> {
        match self {
            Self::Open => None,
            Self::Confirmed { at } | Self::Corrected { at, .. } | Self::Dismissed { at } => {
                Some(*at)
            }
        }
    }
    pub fn valid_text(&self) -> bool {
        match self {
            Self::Corrected { text, .. } => valid_correction(text),
            _ => true,
        }
    }
}

pub fn valid_correction(text: &str) -> bool {
    text.len() <= 262_144
        && text.chars().count() <= 65_536
        && !text.trim().is_empty()
        && !text
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
}

impl ReviewFlag {
    pub fn resolve(
        &self,
        action: &ReviewAction,
        at: UtcInstant,
    ) -> Result<Resolution, ResolveError> {
        if matches!(&self.content, FlagContent::Lexical(flag) if !flag.class.surfaced()) {
            return Err(ResolveError::HiddenFlag);
        }
        let next = match action {
            ReviewAction::Confirm => Resolution::Confirmed { at },
            ReviewAction::Dismiss => Resolution::Dismissed { at },
            ReviewAction::Correct(text) => {
                if matches!(self.content, FlagContent::Diagnostic(_)) {
                    return Err(ResolveError::DiagnosticCorrection);
                }
                if !valid_correction(text) {
                    return Err(ResolveError::InvalidText);
                }
                Resolution::Corrected {
                    text: text.clone(),
                    at,
                }
            }
        };
        let same = match (&self.resolution, &next) {
            (Resolution::Corrected { text: old, .. }, Resolution::Corrected { text: new, .. }) => {
                old == new
            }
            (old, new) => old.kind() == new.kind(),
        };
        Ok(if same { self.resolution.clone() } else { next })
    }
}

#[cfg(test)]
mod tests;
