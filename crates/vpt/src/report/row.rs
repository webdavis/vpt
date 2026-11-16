use crate::style::Verdict;
use serde_json::{Value, json};

#[derive(Clone)]
pub(crate) struct Row {
    pub(crate) name: String,
    pub(crate) verdict: Verdict,
    pub(crate) detail: String,
    pub(crate) advice: Option<String>,
    pub(crate) token_sent: bool,
}

impl Row {
    pub(crate) fn new(
        name: impl Into<String>,
        verdict: Verdict,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            verdict,
            detail: detail.into(),
            advice: None,
            token_sent: false,
        }
    }

    pub(crate) fn ok(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Verdict::Ok, detail)
    }

    pub(crate) fn issue(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Verdict::Issue, detail)
    }

    pub(crate) fn not_run(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Verdict::NotRun, detail)
    }

    pub(crate) fn advice(mut self, text: impl Into<String>) -> Self {
        self.advice = Some(text.into());
        self
    }

    pub(crate) fn token_sent(mut self, sent: bool) -> Self {
        self.token_sent = sent;
        self
    }

    pub(super) fn json(&self) -> Value {
        let mut value = json!({"name": self.name, "result": self.verdict.name(),
            "detail": self.detail, "advice": self.advice});
        if self.token_sent {
            value["token_sent"] = Value::Bool(true);
        }
        value
    }
}
