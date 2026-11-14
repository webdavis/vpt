use super::{SqliteLedger, map, migrations};
use crate::ledger::actions::{check, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::{ActionChange, ActionLedger, LedgerError, QueuedAction, WorkDelta};
use vpt_domain::identity::RecordingId;

impl ActionLedger for SqliteLedger {
    fn pending_actions(&self) -> Result<Vec<QueuedAction>, LedgerError> {
        self.snapshot(pending)
    }
}

fn pending(connection: &Connection) -> Result<Vec<QueuedAction>, LedgerError> {
    if migrations::current(connection)? < 18 {
        return Ok(vec![]);
    }
    let mut statement = connection
        .prepare(
            "SELECT recording, id, digest FROM action_sends WHERE state = 'pending'
             ORDER BY recording, id",
        )
        .map_err(map)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(map)?;
    rows.map(|row| {
        let (recording, id, digest) = row.map_err(map)?;
        let recording = RecordingId::parse(&recording)
            .map_err(|_| LedgerError::Corrupt("invalid saved action sends".into()))?;
        Ok(QueuedAction {
            recording,
            id,
            digest,
        })
    })
    .collect()
}

pub(super) fn apply(connection: &Connection, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(change) = &delta.actions else {
        return Ok(());
    };
    check(change)?;
    match change {
        ActionChange::Queue(actions) => {
            for action in actions {
                connection
                    .execute(
                        "INSERT INTO action_sends (recording, id, digest, state)
                         VALUES (?1, ?2, ?3, 'pending')
                         ON CONFLICT (recording, id) DO UPDATE
                         SET digest = excluded.digest, state = 'pending'
                         WHERE action_sends.digest <> excluded.digest",
                        params![action.recording.as_str(), action.id, action.digest],
                    )
                    .map_err(map)?;
            }
            Ok(())
        }
        ActionChange::Settle { action, refused } => {
            let state = if *refused { "refused" } else { "sent" };
            let changed = connection
                .execute(
                    "UPDATE action_sends SET state = ?1
                     WHERE recording = ?2 AND id = ?3 AND digest = ?4 AND state = 'pending'",
                    params![state, action.recording.as_str(), action.id, action.digest],
                )
                .map_err(map)?;
            if changed == 1 {
                Ok(())
            } else {
                Err(conflict())
            }
        }
        ActionChange::ResendRefused(recording) => {
            connection
                .execute(
                    "UPDATE action_sends SET state = 'pending'
                     WHERE state = 'refused' AND (?1 IS NULL OR recording = ?1)",
                    params![recording.as_ref().map(RecordingId::as_str)],
                )
                .map_err(map)?;
            Ok(())
        }
    }
}
