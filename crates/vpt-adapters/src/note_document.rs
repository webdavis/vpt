use std::ops::Range;
use vpt_domain::identity::RecordingId;
mod body;
pub mod brief;
mod content;
pub use body::body;
mod findings;
mod header;
mod identity;
pub mod metadata;
mod rewrite;
mod tree;
const MAX_HEADER_BYTES: usize = 16 * 1024 * 1024;

pub use vpt_application::ports::NoteFormatError as NoteError;
pub struct NoteFormatter;

impl vpt_application::ports::NoteFormat for NoteFormatter {
    fn claims_digest(
        &self,
        content: &str,
        stage: vpt_application::ports::NoteStage,
        flags: &[vpt_domain::review::ReviewFlag],
    ) -> Result<vpt_domain::digest::Sha256Digest, NoteError> {
        findings::claims_digest(content, stage, flags)
    }
    fn clear_findings(&self, content: &str) -> Result<String, NoteError> {
        findings::clear(content)
    }
    fn content(&self, source: &str) -> Result<String, NoteError> {
        Ok(Document::parse(source)?.content()?.to_owned())
    }
    fn render_findings(
        &self,
        content: &str,
        stage: vpt_application::ports::NoteStage,
        flags: &[vpt_domain::review::ReviewFlag],
    ) -> Result<String, NoteError> {
        findings::render(content, stage, flags)
    }

    fn recording(&self, source: &str) -> Result<Option<RecordingId>, NoteError> {
        identity::recording(source)
    }

    fn rewrite_owned(
        &self,
        source: &str,
        replacement: &str,
        id: &RecordingId,
        stage: vpt_application::ports::NoteStage,
        previous_tags: &[String],
    ) -> Result<String, NoteError> {
        let document = Document::parse(source)?;
        document.check_owner(id, stage.as_str())?;
        document.rewrite(replacement, previous_tags)
    }
}

pub struct Document<'a> {
    pub recording: RecordingId,
    pub schema: u32,
    pub stage: String,
    pub tags: Vec<String>,
    source: &'a str,
    header: Range<usize>,
    body: usize,
    tree: tree::Tree<'a>,
    fields: Vec<Field>,
}

struct Field {
    key: String,
    key_node: usize,
    value_node: usize,
}

impl<'a> Document<'a> {
    pub fn parse(source: &'a str) -> Result<Self, NoteError> {
        let (header, body) = header::locate(source)?;
        let tree = tree::Tree::parse(&source[header.clone()])?;
        let fields = header::fields(&tree)?;
        let get = |key: &str| {
            fields
                .iter()
                .find(|field| field.key == key)
                .map(|field| field.value_node)
                .ok_or(NoteError::Frontmatter)
        };
        let schema = tree.integer(get("vpt-schema")?)?;
        if schema == 0 {
            return Err(NoteError::Frontmatter);
        }
        let recording = RecordingId::parse(tree.string(get("vpt-recording")?)?)
            .map_err(|_| NoteError::Identity)?;
        let stage = tree.string(get("vpt-stage")?)?.to_owned();
        if !matches!(stage.as_str(), "transcript" | "analysis") {
            return Err(NoteError::Stage);
        }
        let tags = fields
            .iter()
            .find(|field| field.key == "tags")
            .map(|field| tree.strings(field.value_node))
            .transpose()?
            .unwrap_or_default();
        vpt_domain::note::managed::Regions::parse(&source[body..])
            .map_err(|_| NoteError::Markers)?;
        Ok(Self {
            source,
            recording,
            schema,
            stage,
            tags,
            header,
            body,
            tree,
            fields,
        })
    }
    pub fn check_owner(&self, id: &RecordingId, stage: &str) -> Result<(), NoteError> {
        if self.schema > 1 {
            return Err(NoteError::UnsupportedSchema(self.schema));
        }
        if self.recording != *id {
            return Err(NoteError::Identity);
        }
        if self.stage != stage {
            return Err(NoteError::Stage);
        }
        Ok(())
    }
    pub fn content(&self) -> Result<&str, NoteError> {
        Ok(
            vpt_domain::note::managed::Regions::parse(&self.source[self.body..])
                .map_err(|_| NoteError::Markers)?
                .content(),
        )
    }
    pub fn rewrite(
        &self,
        replacement: &str,
        previous_tags: &[String],
    ) -> Result<String, NoteError> {
        rewrite::rewrite(self, replacement, previous_tags)
    }
}

#[cfg(test)]
mod tests;
