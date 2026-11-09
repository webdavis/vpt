use super::*;

pub(crate) fn field(output: &mut Writer, key: &str, value: &str) -> Result<(), RenderError> {
    output.literal(key)?;
    output.literal(": ")?;
    scalar(output, value)?;
    output.literal("\n")
}

pub(crate) fn list<'a>(
    output: &mut Writer,
    key: &str,
    values: impl ExactSizeIterator<Item = &'a str>,
) -> Result<(), RenderError> {
    if values.len() > 256 {
        return Err(RenderError::CapacityExceeded);
    }
    output.literal(key)?;
    output.literal(if values.len() == 0 { ": []\n" } else { ":\n" })?;
    for value in values {
        output.literal("  - ")?;
        scalar(output, value)?;
        output.literal("\n")?;
    }
    Ok(())
}

pub(crate) fn scalar(output: &mut Writer, value: &str) -> Result<(), RenderError> {
    if value.len() > 4 * 65_536 || value.chars().count() > 65_536 {
        return Err(RenderError::CapacityExceeded);
    }
    output.literal("\"")?;
    for ch in value.chars() {
        match ch {
            '"' => output.literal("\\\"")?,
            '\\' => output.literal("\\\\")?,
            '\n' => output.literal("\\n")?,
            '\r' => output.literal("\\r")?,
            '\t' => output.literal("\\t")?,
            '<' | '>' | '\u{2028}' | '\u{2029}' | '\u{fffe}' | '\u{ffff}' => {
                output.literal(&format!("\\u{:04x}", ch as u32))?
            }
            ch if ch.is_control() => output.literal(&format!("\\u{:04x}", ch as u32))?,
            _ => output.literal(ch.encode_utf8(&mut [0; 4]))?,
        }
    }
    output.literal("\"")
}
