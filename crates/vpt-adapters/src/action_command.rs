mod expand;
mod fields;
use crate::{OUTPUT_LIMIT, SpawnError, Status, spawn};
use fields::Fields;
use sha2::Digest;
use std::path::Path;
use std::time::Duration;
use vpt_application::{NoteProfile, Settings};
use vpt_domain::{identity::RecordingId, proposal::CheckedAction};

const DEADLINE: Duration = Duration::from_secs(30);

pub struct ActionSend<'a> {
    pub recording: &'a RecordingId,
    pub action: &'a CheckedAction,
    pub id: &'a str,
    pub note: &'a str,
    pub link: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Sent,
    Failed,
    Refused,
}

pub fn action_digest(action: &CheckedAction) -> String {
    let fields = Fields::of(action);
    let mut bytes = Vec::new();
    for field in [fields.text, fields.owner, fields.owner_name, &fields.due] {
        bytes.extend_from_slice(&field.len().to_be_bytes());
        bytes.extend_from_slice(field.as_bytes());
    }
    vpt_domain::digest::Sha256Digest(sha2::Sha256::digest(&bytes).into()).hex()
}

pub fn note_link(note: &Path, profile: NoteProfile) -> String {
    let encoded: String = note
        .to_string_lossy()
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    match profile {
        NoteProfile::Portable => format!("file://{encoded}"),
        NoteProfile::Obsidian => format!("obsidian://open?path={encoded}"),
    }
}

pub fn send_action(settings: &Settings, send: &ActionSend<'_>) -> SendOutcome {
    let fields = Fields::of(send.action);
    let owner_word = settings
        .actions_owner_words
        .iter()
        .find(|(kind, _)| kind == fields.owner)
        .map_or(fields.owner, |(_, word)| word.as_str());
    let moment = match send.action.claim.grounding.sources.first() {
        Some(source) => format!(
            "{}@{}",
            send.recording.as_str(),
            vpt_domain::search::timecode(source.start_secs)
        ),
        None => send.recording.as_str().to_owned(),
    };
    let values = [
        ("{text}", fields.text),
        ("{owner}", fields.owner),
        ("{owner_name}", fields.owner_name),
        ("{owner_word}", owner_word),
        ("{due}", fields.due.as_str()),
        ("{due_date}", fields.due_date),
        ("{due_time}", fields.due_time),
        ("{id}", send.id),
        ("{moment}", &moment),
        ("{recording}", send.recording.as_str()),
        ("{note}", send.note),
        ("{link}", send.link),
    ];
    let Some(argv) = expand::expand(&settings.actions_command, &values, expand::OPTIONAL) else {
        return SendOutcome::Failed;
    };
    let env: Vec<_> = settings
        .actions_env
        .0
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    match spawn::run_with_env(&argv, &[], DEADLINE, OUTPUT_LIMIT, &env)
        .map(|outcome| outcome.status)
    {
        Ok(Status::Exited(0)) => SendOutcome::Sent,
        Ok(Status::Exited(code)) if settings.actions_final_exit_codes.contains(&code) => {
            SendOutcome::Refused
        }
        Ok(_) | Err(SpawnError::NotFound(_) | SpawnError::Io(_)) => SendOutcome::Failed,
    }
}

#[cfg(test)]
mod tests;
