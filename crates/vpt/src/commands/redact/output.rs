use crate::cli::output::Outcome;
use std::collections::BTreeMap;
use vpt_adapters::redaction::{Candidate, Content};
use vpt_application::ports::ReleaseRecord;
pub(super) struct Summary {
    masks: BTreeMap<&'static str, usize>,
    candidates: Vec<Candidate>,
    omitted: usize,
    marked: usize,
}
impl Summary {
    pub fn from_content(content: &Content) -> Self {
        let mut masks = BTreeMap::new();
        for mask in &content.mappings {
            *masks.entry(mask.class.as_str()).or_insert(0) += 1;
        }
        Self {
            masks,
            candidates: content.candidates.clone(),
            omitted: content.omitted,
            marked: content.marked,
        }
    }
}
pub(crate) struct Written {
    pub release: ReleaseRecord,
    pub(super) summary: Summary,
    pub(super) collision: bool,
}
impl Written {
    pub fn fields(&self) -> serde_json::Value {
        let release = &self.release;
        let summary = &self.summary;
        let candidates: Vec<_> = self
            .summary
            .candidates
            .iter()
            .map(|c| serde_json::json!({"token":c.token, "line":c.line}))
            .collect();
        serde_json::json!({
            "id":release.recording.as_str(), "stage":release.stage.as_str(),
            "written":release.destination, "report":release.report_path, "masks":summary.masks,
            "candidates":candidates, "omitted":summary.omitted, "marked":summary.marked,
            "collision":self.collision, "sequence":release.sequence
        })
    }
    pub fn human(&self) -> String {
        let release = &self.release;
        let summary = &self.summary;
        let mut human = format!(
            "Wrote {}\nPrivate report: {}\nOmitted: {}; marked: {}\n",
            release.destination.display(),
            release.report_path.display(),
            summary.omitted,
            summary.marked
        );
        for (class, count) in &summary.masks {
            human.push_str(&format!("Masked {class}: {count}\n"));
        }
        if self.collision {
            human.push_str("Filename collision: used a timestamped name\n");
        }
        for candidate in &summary.candidates {
            human.push_str(&format!(
                "Candidate, line {}: {}\n",
                candidate.line, candidate.token
            ));
        }
        human
    }
    pub fn success(&self) -> Outcome {
        Outcome::Success {
            document: vpt_protocol::result::document("redact", self.fields()),
            human: self.human(),
            completed: vec![self.release.recording.to_string()],
            diagnostics: vec![],
        }
    }
}
