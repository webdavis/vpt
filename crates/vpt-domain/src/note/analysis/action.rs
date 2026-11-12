use super::{RenderError, Writer};
use crate::proposal::{CheckedAction, Owner};

const OWNER: &str = "[owner:: ";
pub const ACTION_FIELDS: &[&str] = &[OWNER];

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
    Ok(())
}
