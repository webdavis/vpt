#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Term,
    Note,
    Email,
    Url,
    Phone,
    Money,
    Number,
}
impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Term => "term",
            Self::Note => "note",
            Self::Email => "email",
            Self::Url => "url",
            Self::Phone => "phone",
            Self::Money => "money",
            Self::Number => "number",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Capacity,
    InvalidMask,
    InvalidText,
    Residue { class: Class, number: Option<usize> },
    Split { class: Class, between: Vec<String> },
}
#[derive(Clone, PartialEq, Eq)]
pub struct Options {
    pub classes: Vec<Class>,
    pub mask: String,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            classes: vec![
                Class::Email,
                Class::Url,
                Class::Phone,
                Class::Number,
                Class::Money,
            ],
            mask: "[{class} {n}]".into(),
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Mapping {
    pub class: Class,
    pub number: usize,
    pub label: String,
    pub value: String,
}
impl std::fmt::Debug for Mapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mapping")
            .field("class", &self.class)
            .field("number", &self.number)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedactionOptions")
            .field("classes", &self.classes)
            .finish_non_exhaustive()
    }
}
pub(super) fn validate_mask(mask: &str) -> Result<(), Error> {
    if mask.len() > 256
        || mask.chars().any(char::is_control)
        || mask.matches("{n}").count() != 1
        || mask.matches("{class}").count() > 1
        || mask
            .replace("{n}", "")
            .replace("{class}", "")
            .contains(['{', '}'])
    {
        return Err(Error::InvalidMask);
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlagPolicy {
    #[default]
    Omit,
    Mark,
}
