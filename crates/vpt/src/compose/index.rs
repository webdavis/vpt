use vpt_adapters::{Roots, note_index::Index};
use vpt_application::Settings;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn read(settings: &Settings, roots: &Roots) -> Result<Index, Box<ErrorDocument>> {
    let paths = match vpt_adapters::config::resolve_vault(settings)
        .map_err(|error| Box::new(super::errors::root_error(&error)))?
    {
        Some(vault) => vec![vault],
        None => vec![
            roots.stores.transcripts.clone(),
            roots.stores.analysis.clone(),
            roots.stores.briefs.clone(),
        ],
    };
    Index::read(&paths).map_err(|error| Box::new(super::errors::store_error(error)))
}
