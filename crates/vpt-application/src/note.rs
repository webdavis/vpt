mod content;
mod locate;
mod relations;
mod render;
mod rewrite;
mod targets;
use crate::{Settings, ports::*};
use std::path::{Path, PathBuf};
use vpt_domain::{
    note::{self, RenderError},
    time::{UtcInstant, UtcOffset},
};

#[derive(Debug)]
pub enum NoteWriteError {
    NoTranscript,
    AnalysisUnavailable,
    Ambiguous(Vec<PathBuf>),
    Foreign(PathBuf),
    Format {
        path: PathBuf,
        error: NoteFormatError,
    },
    Render(RenderError),
    Store(StoreError),
}

#[derive(Clone, Copy)]
pub struct WriteOptions<'a> {
    pub roots: &'a crate::StorePaths,
    pub analysis_content: Option<&'a str>,
    pub regenerate_expired: bool,
}

pub fn prepare(
    input: &NoteInputs,
    settings: &Settings,
    options: WriteOptions<'_>,
    at: UtcInstant,
    offset: UtcOffset,
    stores: &(impl Stores + NoteFiles),
    format: &impl NoteFormat,
) -> Result<WorkDelta, NoteWriteError> {
    if input.review.accepted.is_none() {
        return Err(NoteWriteError::NoTranscript);
    }
    let targets = targets::locate(input, settings, &options, at, offset, stores, format)?;
    let (relations, relation_change) =
        relations::prepare(input, settings.relations.session_gap_minutes, at)?;
    let mut delta = WorkDelta {
        relations: relation_change,
        ..Default::default()
    };
    let (review, contents) = content::prepare(input, settings, &options, &targets, format)?;
    let rendering = render::Context {
        input,
        review: &review,
        relations: &relations,
        settings,
        roots: options.roots,
        format,
    };
    for (target, content) in targets.iter().zip(contents) {
        let counterpart = targets
            .iter()
            .find(|other| other.stage != target.stage)
            .map(|other| other.path.as_path());
        let rendered = render::document(&rendering, target, counterpart, content)?;
        rewrite::append(&mut delta, input, target, rendered, at, stores, format)?;
    }
    Ok(delta)
}
