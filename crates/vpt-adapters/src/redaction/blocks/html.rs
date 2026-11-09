use super::*;
#[derive(Default)]
pub(super) struct Hidden {
    tags: Vec<String>,
}
impl Hidden {
    pub fn active(&self) -> bool {
        !self.tags.is_empty()
    }
    pub fn accept(&mut self, value: &str) -> Result<(), Error> {
        let Some(rest) = value.strip_prefix('<') else {
            return Ok(());
        };
        if rest.starts_with(['!', '?']) {
            return Ok(());
        }
        let closing = rest.starts_with('/');
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
            .map(|ch| ch.to_ascii_lowercase())
            .collect();
        if name.is_empty() {
            return Ok(());
        }
        if closing {
            if let Some(index) = self.tags.iter().rposition(|tag| *tag == name) {
                self.tags.truncate(index);
            }
        } else if !value.trim_end().ends_with("/>")
            && !matches!(
                name.as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            )
        {
            if self.tags.len() >= 128 || name.len() > 256 {
                return Err(Error::Capacity);
            }
            self.tags.push(name);
        }
        Ok(())
    }
}
