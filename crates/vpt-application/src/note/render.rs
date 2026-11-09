use super::*;
mod links;
mod relations;
use crate::{LinkStyle, NoteProfile};
use vpt_domain::{
    note::document::{self, Profile, RecordingNote},
    review::{FlagContent, Resolution},
};

pub(super) struct Rendered {
    pub text: String,
    pub content: String,
    pub managed_tags: Vec<String>,
}

pub(super) struct Context<'a, F> {
    pub input: &'a NoteInputs,
    pub review: &'a ReviewSnapshot,
    pub relations: &'a [NoteRelation],
    pub settings: &'a Settings,
    pub roots: &'a crate::StorePaths,
    pub format: &'a F,
}

pub(super) fn document(
    context: &Context<'_, impl NoteFormat>,
    target: &targets::Target<'_>,
    counterpart: Option<&Path>,
    content: String,
) -> Result<Rendered, NoteWriteError> {
    let Context {
        input,
        review,
        relations,
        settings,
        roots,
        format,
    } = context;
    let accepted = review
        .accepted
        .as_ref()
        .ok_or(NoteWriteError::NoTranscript)?;
    let content =
        if target.stage == NoteStage::Analysis && review.stale_artifacts.contains(&target.stage) {
            format.clear_findings(&content)
        } else {
            format.render_findings(&content, target.stage, &review.flags)
        }
        .map_err(|error| NoteWriteError::Format {
            path: target.path.clone(),
            error,
        })?;
    let engines = std::iter::once(&accepted.transcript.engine)
        .chain(accepted.checker_engine.as_ref())
        .map(|engine| format!("{}:{}", engine.name, engine.model))
        .collect::<Vec<_>>();
    let previous = input.notes.iter().find(|note| note.stage == target.stage);
    let tags = input.tags.as_ref().map_or_else(
        || {
            previous
                .map(|note| note.managed_tags.clone())
                .unwrap_or_default()
        },
        |tags| {
            tags.iter()
                .filter(|tag| tag.state == vpt_domain::tags::State::Confirmed)
                .map(|tag| tag.name.clone())
                .collect()
        },
    );
    let suggested_tags: Vec<_> = input
        .tags
        .iter()
        .flatten()
        .filter(|tag| tag.state == vpt_domain::tags::State::Suggested)
        .map(|tag| tag.name.clone())
        .collect();
    let open_flags = review
        .flags
        .iter()
        .filter(|flag| {
            flag.resolution == Resolution::Open
                && (matches!(&flag.content, FlagContent::Lexical(flag) if flag.class.surfaced())
                    || matches!(flag.content, FlagContent::Verification(_)))
        })
        .count();
    let diagnostics = review
        .flags
        .iter()
        .filter(|flag| {
            flag.resolution == Resolution::Open
                && matches!(flag.content, FlagContent::Diagnostic(_))
        })
        .count();
    let wiki_root = links::wiki::root(settings, roots);
    let (reference, hub) =
        links::references(input, settings, &target.path, target.stage, wiki_root)?;
    let mut links = format!("- Audio: {reference}\n");
    links.push_str(&links::counterpart(
        settings,
        &target.path,
        target.stage,
        counterpart,
        wiki_root,
    )?);
    links.push_str(&relations::render(
        relations,
        settings,
        &target.path,
        wiki_root,
    )?);
    let text = document::create(&RecordingNote {
        stage: target.stage,
        recording: &input.record.id,
        filename: target
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(NoteWriteError::Render(RenderError::InvalidName))?,
        captured_at: input.record.captured_at,
        offset: input.record.captured_offset,
        duration_secs: input.record.duration_secs,
        language: &accepted.requested_language,
        engines: &engines,
        readability: settings.readability_enabled,
        open_flags,
        diagnostics,
        tags: &tags,
        suggested_tags: &suggested_tags,
        content: &content,
        links: &links,
        profile: match settings.note.profile {
            NoteProfile::Portable => Profile::Portable,
            NoteProfile::Obsidian => Profile::Obsidian {
                title: input.record.title.as_deref(),
                reference: &reference,
                hub: &hub,
                first_written: target.first_written,
                write_offset: target.write_offset,
            },
        },
    })
    .map_err(NoteWriteError::Render)?;
    Ok(Rendered {
        text,
        content,
        managed_tags: tags,
    })
}
