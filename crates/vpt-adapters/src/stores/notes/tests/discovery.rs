use super::*;
use crate::note_document::NoteFormatter;
use vpt_application::ports::{NoteFormat, NoteFormatError, NoteInputs, NoteStage, ReviewSnapshot};
use vpt_domain::identity::RecordingId;

struct ChangingFormat<'a> {
    root: &'a Path,
    duplicate: bool,
}
impl NoteFormat for ChangingFormat<'_> {
    fn claims_digest(
        &self,
        content: &str,
        stage: NoteStage,
        flags: &[vpt_domain::review::ReviewFlag],
    ) -> Result<vpt_domain::digest::Sha256Digest, NoteFormatError> {
        NoteFormatter.claims_digest(content, stage, flags)
    }
    fn clear_findings(&self, content: &str) -> Result<String, NoteFormatError> {
        NoteFormatter.clear_findings(content)
    }
    fn content(&self, source: &str) -> Result<String, NoteFormatError> {
        NoteFormatter.content(source)
    }
    fn render_findings(
        &self,
        content: &str,
        stage: NoteStage,
        flags: &[vpt_domain::review::ReviewFlag],
    ) -> Result<String, NoteFormatError> {
        NoteFormatter.render_findings(content, stage, flags)
    }

    fn recording(&self, source: &str) -> Result<Option<RecordingId>, NoteFormatError> {
        if self.duplicate {
            std::fs::write(self.root.join("second.md"), source).unwrap();
        } else {
            std::fs::write(
                self.root.join("renamed.md"),
                format!("{source}\nOperator edit"),
            )
            .unwrap();
        }
        NoteFormatter.recording(source)
    }
    fn rewrite_owned(
        &self,
        source: &str,
        replacement: &str,
        id: &RecordingId,
        stage: NoteStage,
        tags: &[String],
    ) -> Result<String, NoteFormatError> {
        NoteFormatter.rewrite_owned(source, replacement, id, stage, tags)
    }
}

#[test]
fn rename_discovery_refuses_claimants_added_or_edited_during_the_scan() {
    for duplicate in [false, true] {
        let (_temp, root, stores) = fixture();
        let mut note = crate::ledger::contract::notes::artifact();
        note.target = root.join("missing.md");
        std::fs::write(root.join("renamed.md"), &note.text).unwrap();
        let input = NoteInputs {
            proposal: None,
            derived_mentions: None,
            mention_ambiguities: vec![],
            relations: vec![],
            predecessor: None,
            tags: None,
            record: crate::ledger::contract::record(81),
            review: ReviewSnapshot {
                verified_claims: vec![],
                stale_findings: vec![],
                stale_artifacts: vec![],
                accepted: Some(crate::ledger::contract::transcript::fixture()),
                flags: vec![],
            },
            notes: vec![note.clone()],
        };
        let mut settings = crate::config::from_table(
            &crate::config::load_text("config_version = 1").unwrap(),
            &root,
        )
        .unwrap();
        settings.stores.transcripts = root.clone();
        let result = vpt_application::note::prepare(
            &input,
            &settings,
            vpt_application::note::WriteOptions {
                analysis_content: None,
                roots: &settings.stores,
                regenerate_expired: true,
            },
            note.updated_at,
            note.write_offset,
            &stores,
            &ChangingFormat {
                root: &root,
                duplicate,
            },
        );
        assert!(
            matches!(result, Err(vpt_application::note::NoteWriteError::Store(_))),
            "{result:?}"
        );
        assert!(!note.target.exists());
    }
}
