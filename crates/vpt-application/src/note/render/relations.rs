use super::*;
use vpt_domain::relations::{Kind, Provenance, State};

pub(super) fn render(
    relations: &[NoteRelation],
    settings: &Settings,
    target: &Path,
    wiki_root: Option<&Path>,
) -> Result<String, NoteWriteError> {
    if relations.len() > 65_536 {
        return Err(error(RenderError::CapacityExceeded));
    }
    let origin = target
        .parent()
        .ok_or_else(|| error(RenderError::InvalidName))?;
    let mut relations: Vec<_> = relations.iter().collect();
    relations.sort_by(|a, b| {
        (a.relation.kind, a.relation.target.as_str())
            .cmp(&(b.relation.kind, b.relation.target.as_str()))
    });
    let mut suggestions = settings.relations.max_suggested;
    let mut result = String::new();
    for item in relations {
        let relation = &item.relation;
        if relation.state == State::Rejected {
            continue;
        }
        if relation.kind == Kind::Related && relation.state == State::Suggested {
            if suggestions == 0 {
                continue;
            }
            suggestions -= 1;
        }
        let label = match &item.path {
            Some(path) => path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| error(RenderError::InvalidName))?,
            None => relation.target.as_str(),
        };
        let link = match &item.path {
            Some(path) => {
                if path.as_os_str().len() > 262_144 {
                    return Err(error(RenderError::CapacityExceeded));
                }
                links::reference(settings.note.link_style, label, path, origin, wiki_root)?
            }
            None => format!("{} (note unavailable)", text(label)?),
        };
        let kind = match relation.kind {
            Kind::Continues => "Continues",
            Kind::Mentions => "Mentions",
            Kind::Related => "Related",
            Kind::Supersedes => "Supersedes",
        };
        append(&mut result, &format!("- {kind}: {link}"))?;
        if let Provenance::Proposal { by, reason, .. } = &relation.provenance {
            let state = if relation.state == State::Suggested {
                "suggested"
            } else {
                "proposed"
            };
            append(
                &mut result,
                &format!(" ({state} by {}: {})", text(by)?, text(reason)?),
            )?;
        } else if relation.kind == Kind::Related {
            append(&mut result, " (confirmed manually)")?;
        }
        append(&mut result, "\n")?;
    }
    Ok(result)
}
fn text(value: &str) -> Result<String, NoteWriteError> {
    if value.len() > 262_144 || value.chars().count() > 65_536 {
        return Err(error(RenderError::CapacityExceeded));
    }
    note::escape_text(value).map_err(error)
}
fn append(output: &mut String, value: &str) -> Result<(), NoteWriteError> {
    if value.len() > note::MAX_RENDERED - output.len() {
        return Err(error(RenderError::CapacityExceeded));
    }
    output.push_str(value);
    Ok(())
}
fn error(error: RenderError) -> NoteWriteError {
    NoteWriteError::Render(error)
}
