use super::{Field, NoteError, header, rewrite, tree};
use std::ops::Range;
use vpt_domain::{brief::markdown::Region, note::RenderError, occasion::OccasionId};
mod update;

pub struct Document<'a> {
    pub id: OccasionId,
    pub schema: u32,
    pub tags: Vec<String>,
    source: &'a str,
    header: Range<usize>,
    body: usize,
    tree: tree::Tree<'a>,
    fields: Vec<Field>,
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
        let schema = tree.integer(get("vptSchema")?)?;
        if schema == 0 {
            return Err(NoteError::Frontmatter);
        }
        if tree.string(get("vptKind")?)? != "brief" {
            return Err(NoteError::Stage);
        }
        let id = OccasionId::parse(tree.string(get("vptOccasion")?)?)
            .map_err(|_| NoteError::Identity)?;
        let tags = fields
            .iter()
            .find(|field| field.key == "tags")
            .map(|field| tree.strings(field.value_node))
            .transpose()?
            .unwrap_or_default();
        Region::parse(&source[body..]).map_err(format_error)?;
        Ok(Self {
            source,
            header,
            body,
            tree,
            fields,
            id,
            schema,
            tags,
        })
    }
    pub fn check_owner(&self, id: &OccasionId) -> Result<(), NoteError> {
        if self.schema > 1 {
            return Err(NoteError::UnsupportedSchema(self.schema));
        }
        if self.id != *id {
            return Err(NoteError::Identity);
        }
        Ok(())
    }
    pub fn content(&self) -> Result<&'a str, NoteError> {
        Ok(Region::parse(&self.source[self.body..])
            .map_err(format_error)?
            .content())
    }
    pub fn rewrite(
        &self,
        replacement: &str,
        previous_tags: &[String],
    ) -> Result<String, NoteError> {
        update::rewrite(self, replacement, previous_tags)
    }
    fn header_view(&self) -> rewrite::View<'a, '_> {
        rewrite::View {
            source: self.source,
            header: self.header.clone(),
            tree: &self.tree,
            fields: &self.fields,
            tags: &self.tags,
        }
    }
}
fn format_error(error: RenderError) -> NoteError {
    match error {
        RenderError::CapacityExceeded => NoteError::CapacityExceeded,
        _ => NoteError::Markers,
    }
}

#[cfg(test)]
mod tests;
