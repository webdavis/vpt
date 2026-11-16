use super::Verdict;

/// Builds what one command reports at a terminal: marked rows, advice under them, and a closing
/// "Needs attention" section only when a row warned or failed.
#[derive(Default)]
pub(crate) struct Summary {
    text: String,
    attention: Vec<(Verdict, String)>,
}

impl Summary {
    pub(crate) fn row(&mut self, verdict: Verdict, name: &str, detail: &str) -> &mut Self {
        self.text.push_str(&super::row(verdict, name, detail));
        self.note(verdict, format!("{name}: {detail}"))
    }

    pub(crate) fn mark(&mut self, verdict: Verdict, text: &str) -> &mut Self {
        self.text.push_str(&super::mark(verdict, text));
        self.note(verdict, text.to_owned())
    }

    pub(crate) fn advice(&mut self, text: &str) -> &mut Self {
        self.text.push_str(&super::advice(text));
        self
    }

    pub(crate) fn finish(self, nothing: &str) -> String {
        if self.text.is_empty() {
            return super::mark(Verdict::NotRun, nothing);
        }
        if self.attention.is_empty() {
            return self.text;
        }
        self.text + &super::needs_attention(&self.attention)
    }

    fn note(&mut self, verdict: Verdict, entry: String) -> &mut Self {
        if matches!(verdict, Verdict::Warning | Verdict::Issue) {
            self.attention.push((verdict, entry));
        }
        self
    }
}

pub(crate) fn counted(count: u64, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        count => format!("{count} {many}"),
    }
}

#[cfg(test)]
mod tests;
