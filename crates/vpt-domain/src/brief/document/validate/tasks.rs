use super::*;
use crate::occasion::OccasionSource;
use std::collections::BTreeSet;

pub(super) fn check(document: &Document<'_>, bytes: &mut usize) -> Result<(), DocumentError> {
    if document.tasks.len() > 256 {
        return Err(DocumentError::CapacityExceeded);
    }
    if !document.tasks.is_empty() && document.context.tasks != ContextStatus::Present {
        return Err(DocumentError::Invalid);
    }
    let mut ids = BTreeSet::new();
    for task in document.tasks {
        if task.source == OccasionSource::Manual
            || !ids.insert((task.source.as_str(), task.id.as_str()))
        {
            return Err(DocumentError::Invalid);
        }
        for value in [&task.project, &task.id, &task.content]
            .into_iter()
            .chain(task.due.iter())
        {
            if value.trim().is_empty() {
                return Err(DocumentError::Invalid);
            }
            text(value, bytes)?;
        }
    }
    Ok(())
}
