use std::{collections::HashMap, path::PathBuf};
use vpt_application::ports::{LedgerError, NoteRelation};
use vpt_domain::{
    identity::RecordingId,
    relations::{Relation, Target},
};

pub(in crate::ledger) fn resolve(
    relations: Vec<Relation>,
    mut path: impl FnMut(&RecordingId) -> Result<Option<PathBuf>, LedgerError>,
) -> Result<Vec<NoteRelation>, LedgerError> {
    let mut paths = HashMap::new();
    let mut remaining = super::MAX_BYTES;
    let mut result = Vec::new();
    for relation in relations {
        let target = match &relation.target {
            Target::Note(path) => Some(PathBuf::from(path)),
            Target::Recording(id) => {
                if !paths.contains_key(id) {
                    paths.insert(id.clone(), path(id)?);
                }
                paths.get(id).cloned().flatten()
            }
        };
        remaining = remaining
            .checked_sub(target.as_ref().map_or(0, |p| p.as_os_str().len()))
            .ok_or_else(super::invalid)?;
        result.push(NoteRelation {
            relation,
            path: target,
        });
    }
    Ok(result)
}
