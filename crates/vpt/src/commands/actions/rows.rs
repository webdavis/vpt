use crate::compose::{actions::queued, errors::ledger_error};
use serde_json::{Value, json};
use vpt_application::ports::{ActionLedger, LedgerError, ProposalLedger, SendState};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) struct Row {
    pub recording: RecordingId,
    pub id: String,
    pub state: SendState,
    pub text: Option<String>,
}

impl Row {
    pub(super) fn json(&self) -> Value {
        json!({"recording": self.recording.as_str(), "id": self.id, "state": self.state.name(), "text": self.text})
    }
}

pub(super) fn recording(id: Option<&str>) -> Result<Option<RecordingId>, Box<ErrorDocument>> {
    id.map(|id| {
        RecordingId::parse(id)
            .map_err(|_| Box::new(ErrorDocument::new(ErrorKind::Usage, "invalid recording id")))
    })
    .transpose()
}

/// The saved follow-ups in these states, one recording's or every recording's, ordered by
/// recording and id. A follow-up carries its text while the recording's accepted analysis still
/// holds it unchanged.
pub(super) fn read(
    ledger: &(impl ActionLedger + ProposalLedger),
    only: Option<&RecordingId>,
    states: &[SendState],
) -> Result<Vec<Row>, Box<ErrorDocument>> {
    let error = |error: LedgerError| Box::new(ledger_error(error));
    let mut rows = Vec::new();
    for state in states {
        for action in ledger.actions_in(*state).map_err(error)? {
            if only.is_none_or(|only| *only == action.recording) {
                rows.push((action, *state));
            }
        }
    }
    rows.sort_by(|(a, _), (b, _)| {
        (a.recording.as_str(), &a.id).cmp(&(b.recording.as_str(), &b.id))
    });
    let mut read = Vec::new();
    for (action, state) in rows {
        let proposal = ledger.accepted_proposal(&action.recording).map_err(error)?;
        let text = proposal.and_then(|accepted| {
            queued(&action.recording, &accepted.proposal)
                .into_iter()
                .zip(accepted.proposal.actions)
                .find(|(current, _)| *current == action)
                .map(|(_, checked)| checked.claim.claim.text)
        });
        read.push(Row {
            recording: action.recording,
            id: action.id,
            state,
            text,
        });
    }
    Ok(read)
}
