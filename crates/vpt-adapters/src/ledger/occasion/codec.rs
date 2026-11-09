use super::validate::invalid;
use serde::{Deserialize, Serialize};
use vpt_application::ports::LedgerError;
use vpt_domain::occasion::Participant;
const MAX: usize = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedParticipant {
    name: String,
    email: Option<String>,
}

pub(crate) fn participants(values: &[Participant]) -> Result<String, LedgerError> {
    let saved: Vec<_> = values
        .iter()
        .map(|p| SavedParticipant {
            name: p.name.clone(),
            email: p.email.clone(),
        })
        .collect();
    super::super::json::encode(&saved, MAX).map_err(|_| invalid())
}
pub(crate) fn read_participants(text: &str) -> Result<Vec<Participant>, LedgerError> {
    if text.len() > MAX {
        return Err(invalid());
    }
    let saved: Vec<SavedParticipant> = serde_json::from_str(text).map_err(|_| invalid())?;
    if saved.len() > 256 {
        return Err(invalid());
    }
    Ok(saved
        .into_iter()
        .map(|p| Participant {
            name: p.name,
            email: p.email,
        })
        .collect())
}
pub(crate) fn tags(values: &[String]) -> Result<String, LedgerError> {
    super::super::json::encode(&values, MAX).map_err(|_| invalid())
}
pub(crate) fn read_tags(text: &str) -> Result<Vec<String>, LedgerError> {
    if text.len() > MAX {
        return Err(invalid());
    }
    let values: Vec<String> = serde_json::from_str(text).map_err(|_| invalid())?;
    if values.len() > 256 {
        return Err(invalid());
    }
    Ok(values)
}
