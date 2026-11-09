use std::path::PathBuf;
use vpt_domain::history::EventCounts;

pub(super) struct Published {
    pub(super) note: Option<PathBuf>,
    pub(super) counts: EventCounts,
}
impl Published {
    pub(super) fn attach(
        self,
        mut outcome: crate::cli::output::Outcome,
    ) -> crate::cli::output::Outcome {
        if let crate::cli::output::Outcome::Success {
            document, human, ..
        } = &mut outcome
        {
            document["note"] = serde_json::json!(self.note);
            document["flags"] = serde_json::json!({
                "numeric": self.counts.numeric, "proper_noun": self.counts.proper_noun,
                "low_confidence": self.counts.low_confidence, "formatting": self.counts.formatting,
                "other": self.counts.other, "agreed_unverified": self.counts.agreed_unverified,
                "unsupported": self.counts.unsupported, "diagnostics": self.counts.diagnostics,
            });
            if let Some(note) = self.note {
                human.push_str(&format!("Wrote {}\n", note.display()));
            }
        }
        outcome
    }
}
