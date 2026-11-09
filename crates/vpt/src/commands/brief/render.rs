use super::*;
use vpt_adapters::{SystemClock, note_document::brief::Document};
use vpt_application::{NoteProfile, ports::*};
use vpt_domain::{
    brief::{document::Snapshot, markdown},
    digest::Sha256Digest,
    note::document::Profile,
};

pub(super) fn run(
    snapshot: Snapshot,
    previous: Option<&BriefArtifact>,
    pin: Option<&Path>,
    settings: &Settings,
    roots: &Roots,
) -> Result<(BriefArtifact, Option<Sha256Digest>), Box<ErrorDocument>> {
    let origin = &roots.stores.briefs;
    let filename = markdown::filename(&snapshot.document(), settings.note.slug_max_chars as usize)
        .map_err(|_| invalid())?;
    let target = pin
        .map(Path::to_path_buf)
        .unwrap_or_else(|| origin.join(filename));
    if target.parent() != Some(origin.as_path()) {
        return Err(refused());
    }
    let filename = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(invalid)?;
    let first_written = previous.map_or(snapshot.generated_at, |row| row.first_written);
    let write_offset = previous.map_or_else(
        || SystemClock.offset_at(first_written),
        |row| row.write_offset,
    );
    let wiki_root = settings.note.wiki_root(&roots.stores);
    let hub = if settings.note.profile == NoteProfile::Obsidian {
        let root = wiki_root.ok_or_else(invalid)?;
        let name = &settings.note.obsidian.hub_briefs;
        vpt_domain::note::links::reference(
            settings.note.link_style,
            name,
            &root.join(format!("{name}.md")),
            origin,
            wiki_root,
        )
        .map_err(|_| invalid())?
    } else {
        String::new()
    };
    let text = markdown::create(&markdown::Note {
        document: snapshot.document(),
        filename,
        origin,
        wiki_root,
        link_style: settings.note.link_style,
        profile: if settings.note.profile == NoteProfile::Portable {
            Profile::Portable
        } else {
            Profile::Obsidian {
                title: Some(&snapshot.occasion.title),
                reference: "",
                hub: &hub,
                first_written,
                write_offset,
            }
        },
    })
    .map_err(|_| invalid())?;
    let current = match vpt_adapters::RootDir::open(origin) {
        Ok(_) => stores(roots)?
            .read_note(&target)
            .map_err(|error| Box::new(crate::compose::errors::store_error(error)))?,
        Err(vpt_adapters::ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => None,
        Err(error) => return Err(Box::new(crate::compose::errors::contained_error(error))),
    };
    let expected = current.as_ref().map(|row| row.digest);
    let text = match current {
        Some(current) => {
            let old = previous.ok_or_else(refused)?;
            let parsed = Document::parse(&current.text).map_err(|_| refused())?;
            parsed.check_owner(&snapshot.id).map_err(|_| refused())?;
            parsed
                .rewrite(&text, &old.managed_tags)
                .map_err(|_| refused())?
        }
        None => text,
    };
    let tags = snapshot.occasion.tags.clone();
    let artifact = vpt_adapters::prepare_brief_artifact(
        snapshot,
        target,
        text,
        tags,
        first_written,
        write_offset,
    )
    .map_err(ledger_error)?;
    Ok((artifact, expected))
}
fn refused() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(ErrorKind::Refused, "brief target cannot be rewritten")
            .rule("brief_ownership"),
    )
}
