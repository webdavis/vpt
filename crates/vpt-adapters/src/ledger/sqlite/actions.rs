use super::{SqliteLedger, map, migrations};
use crate::ledger::actions::{check, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::{ActionChange, ActionLedger, LedgerError, QueuedAction, WorkDelta};
use vpt_domain::identity::RecordingId;

impl ActionLedger for SqliteLedger {
    fn pending_actions(&self) -> Result<Vec<QueuedAction>, LedgerError> {
        self.snapshot(|connection| in_state(connection, "pending"))
    }

    fn withdrawing_actions(&self) -> Result<Vec<QueuedAction>, LedgerError> {
        self.snapshot(|connection| in_state(connection, "withdrawing"))
    }
}

fn in_state(connection: &Connection, state: &str) -> Result<Vec<QueuedAction>, LedgerError> {
    if migrations::current(connection)? < 19 {
        return Ok(vec![]);
    }
    let mut statement = connection
        .prepare(
            "SELECT recording, id, digest FROM action_sends WHERE state = ?1
             ORDER BY recording, id",
        )
        .map_err(map)?;
    let rows = statement
        .query_map([state], |row| {
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
    let execute =
        |sql: &str, values: &[&dyn rusqlite::ToSql]| connection.execute(sql, values).map_err(map);
    match change {
        ActionChange::Queue {
            recording,
            actions,
            dropped,
        } => {
            for action in actions {
                execute(
                    "INSERT INTO action_sends (recording, id, digest, state, attempted)
                     VALUES (?1, ?2, ?3, 'pending', 0)
                     ON CONFLICT (recording, id) DO UPDATE
                     SET digest = excluded.digest, state = 'pending'
                     WHERE action_sends.digest <> excluded.digest
                        OR action_sends.state = 'withdrawing'",
                    params![action.recording.as_str(), action.id, action.digest],
                )?;
            }
            for id in dropped {
                execute(
                    "DELETE FROM action_sends WHERE recording = ?1 AND id = ?2 AND attempted = 0",
                    params![recording.as_str(), id],
                )?;
                execute(
                    "UPDATE action_sends SET state = 'withdrawing' WHERE recording = ?1 AND id = ?2",
                    params![recording.as_str(), id],
                )?;
            }
            Ok(())
        }
        ActionChange::Attempt(action) => exactly_one(execute(
            "UPDATE action_sends SET attempted = 1
             WHERE recording = ?1 AND id = ?2 AND digest = ?3 AND state = 'pending'",
            params![action.recording.as_str(), action.id, action.digest],
        )?),
        ActionChange::Settle { action, refused } => {
            let state = if *refused { "refused" } else { "sent" };
            exactly_one(execute(
                "UPDATE action_sends SET state = ?1
                 WHERE recording = ?2 AND id = ?3 AND digest = ?4 AND state = 'pending'",
                params![state, action.recording.as_str(), action.id, action.digest],
            )?)
        }
        ActionChange::ResendRefused(recording) => {
            execute(
                "UPDATE action_sends SET state = 'pending'
                 WHERE state = 'refused' AND (?1 IS NULL OR recording = ?1)",
                params![recording.as_ref().map(RecordingId::as_str)],
            )?;
            Ok(())
        }
        ActionChange::Withdrawn(action) => exactly_one(execute(
            "DELETE FROM action_sends
             WHERE recording = ?1 AND id = ?2 AND digest = ?3 AND state = 'withdrawing'",
            params![action.recording.as_str(), action.id, action.digest],
        )?),
    }
}

fn exactly_one(changed: usize) -> Result<(), LedgerError> {
    if changed == 1 {
        Ok(())
    } else {
        Err(conflict())
    }
}
