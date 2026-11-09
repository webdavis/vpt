use super::*;

pub(in crate::note::render) fn root<'a>(
    settings: &'a Settings,
    stores: &'a crate::StorePaths,
) -> Option<&'a Path> {
    settings.note.wiki_root(stores)
}
