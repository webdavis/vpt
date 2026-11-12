use super::{RenderError, Writer};
use crate::proposal::{CheckedAction, Owner};

const OWNER: &str = "[owner:: ";
const DUE: &str = "[due:: ";
pub const ACTION_FIELDS: &[&str] = &[OWNER, DUE];

pub(super) fn fields(output: &mut Writer, action: &CheckedAction) -> Result<(), RenderError> {
    if let Some(owner) = &action.owner {
        output.literal(" ")?;
        output.literal(OWNER)?;
        match owner {
            Owner::Me => output.literal("me")?,
            Owner::Other(Some(name)) => output.text(name)?,
            Owner::Other(None) => output.literal("someone else")?,
        }
        output.literal("]")?;
    }
    if let Some(due) = action.due.as_ref().filter(|due| !due.withheld()) {
        output.literal(" ")?;
        output.literal(DUE)?;
        output.literal(&due.due.date)?;
        if let Some(time) = &due.due.time {
            output.literal(" ")?;
            output.literal(time)?;
        }
        output.literal("]")?;
    }
    Ok(())
}
