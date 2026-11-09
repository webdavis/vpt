use vpt_domain::redaction::{FlagPolicy, Options};
#[derive(Clone, PartialEq, Eq)]
pub struct ShareSettings {
    pub automatic: bool,
    pub source_line: String,
    pub include_date: bool,
    pub keep_timecodes: bool,
    pub deny_sections: Vec<String>,
    pub redact: Options,
    pub flagged_spans: FlagPolicy,
}
impl Default for ShareSettings {
    fn default() -> Self {
        Self {
            automatic: false,
            source_line: "redacted extract, not a verbatim record".into(),
            include_date: false,
            keep_timecodes: false,
            deny_sections: vec![],
            redact: Options::default(),
            flagged_spans: FlagPolicy::Omit,
        }
    }
}
impl std::fmt::Debug for ShareSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShareSettings")
            .field("automatic", &self.automatic)
            .field("include_date", &self.include_date)
            .field("keep_timecodes", &self.keep_timecodes)
            .field("denied_sections", &self.deny_sections.len())
            .field("redact", &self.redact)
            .field("flagged_spans", &self.flagged_spans)
            .finish_non_exhaustive()
    }
}
