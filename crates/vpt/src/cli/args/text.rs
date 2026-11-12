use super::UsageError;

#[derive(PartialEq, Eq)]
pub struct PrivateText(String);
impl PrivateText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub(super) fn new(text: String) -> Self {
        Self(text)
    }
}
impl std::fmt::Debug for PrivateText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivateText")
    }
}

#[derive(Default)]
pub(super) struct Values {
    pub term: Option<PrivateText>,
    pub correction: Option<PrivateText>,
    pub tags: Vec<PrivateText>,
    pub title: Option<PrivateText>,
    pub participants: Vec<PrivateText>,
    pub reject_tag: Option<PrivateText>,
    pub relation: Option<PrivateText>,
}

pub(super) fn take(words: &mut Vec<String>) -> Result<Values, UsageError> {
    let mut values = Values::default();
    let mut index = 0;
    while index < words.len() {
        if matches!(words[index].as_str(), "--tag" | "--participant") {
            let flag = words.remove(index);
            if index == words.len() {
                return Err(UsageError(format!("{flag} needs a value")));
            }
            let items = if flag == "--tag" {
                &mut values.tags
            } else {
                &mut values.participants
            };
            if items.len() == 256 {
                return Err(UsageError(format!("{flag} exceeds capacity")));
            }
            items.push(PrivateText(words.remove(index)));
            continue;
        }
        let slot = match words[index].as_str() {
            "--term" => &mut values.term,
            "--correct" => &mut values.correction,
            "--title" => &mut values.title,
            "--reject-tag" => &mut values.reject_tag,
            "--relation" => &mut values.relation,
            _ => {
                index += 1;
                continue;
            }
        };
        let flag = words.remove(index);
        if slot.is_some() {
            return Err(UsageError(format!("{flag} may only appear once")));
        }
        if index == words.len() {
            return Err(UsageError(format!("{flag} needs a value")));
        }
        *slot = Some(PrivateText(words.remove(index)));
    }
    Ok(values)
}
