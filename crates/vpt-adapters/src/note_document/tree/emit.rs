use super::*;

impl Tree<'_> {
    pub fn flow(&self, id: usize) -> Result<String, NoteError> {
        let mut output = String::new();
        self.emit(id, &mut output, &mut 65_536, 0)?;
        Ok(output)
    }

    fn emit(
        &self,
        id: usize,
        output: &mut String,
        remaining: &mut usize,
        depth: usize,
    ) -> Result<(), NoteError> {
        if depth == 64 {
            return Err(NoteError::CapacityExceeded);
        }
        *remaining = remaining
            .checked_sub(1)
            .ok_or(NoteError::CapacityExceeded)?;
        let node = self.resolve(id)?;
        match &node.kind {
            Kind::Scalar { text, style, tag } => {
                if let Some(tag) = tag {
                    append(output, "!<")?;
                    for byte in tag.bytes() {
                        if byte.is_ascii_alphanumeric() || b"-._~:/,".contains(&byte) {
                            append(
                                output,
                                std::str::from_utf8(&[byte]).map_err(|_| NoteError::Frontmatter)?,
                            )?;
                        } else {
                            append(output, &format!("%{byte:02X}"))?;
                        }
                    }
                    append(output, "> ")?;
                }
                if tag.is_none() && *style == ScalarStyle::Plain && plain_literal(text) {
                    append(output, text)?;
                } else {
                    append(output, &quote(text)?)?;
                }
            }
            Kind::Sequence(children) | Kind::Mapping(children) => {
                let map = matches!(node.kind, Kind::Mapping(_));
                append(output, if map { "{" } else { "[" })?;
                for (index, child) in children.iter().enumerate() {
                    if index > 0 {
                        append(output, if map && index % 2 == 1 { ": " } else { ", " })?;
                    }
                    self.emit(*child, output, remaining, depth + 1)?;
                }
                append(output, if map { "}" } else { "]" })?;
            }
            Kind::Alias(_) => return Err(NoteError::Frontmatter),
        }
        Ok(())
    }
}

pub(in crate::note_document) fn string_list(values: &[&str]) -> Result<String, NoteError> {
    let mut output = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            append(&mut output, ", ")?;
        }
        append(&mut output, &quote(value)?)?;
    }
    append(&mut output, "]")?;
    Ok(output)
}

fn plain_literal(text: &str) -> bool {
    matches!(
        text,
        "null"
            | "Null"
            | "NULL"
            | "~"
            | "true"
            | "True"
            | "TRUE"
            | "false"
            | "False"
            | "FALSE"
            | ".inf"
            | ".Inf"
            | ".INF"
            | ".nan"
            | ".NaN"
            | ".NAN"
            | "-.inf"
            | "+.inf"
    ) || text.parse::<f64>().is_ok()
        || text
            .strip_prefix("0x")
            .is_some_and(|digits| u64::from_str_radix(digits, 16).is_ok())
        || text
            .strip_prefix("0o")
            .is_some_and(|digits| u64::from_str_radix(digits, 8).is_ok())
}

pub(in crate::note_document) fn quote(text: &str) -> Result<String, NoteError> {
    serde_json::to_string(text)
        .map(|encoded| {
            encoded
                .replace('<', "\\u003c")
                .replace('>', "\\u003e")
                .replace('\u{85}', "\\u0085")
                .replace('\u{2028}', "\\u2028")
                .replace('\u{2029}', "\\u2029")
                .replace('\u{fffe}', "\\ufffe")
                .replace('\u{ffff}', "\\uffff")
        })
        .map_err(|_| NoteError::Frontmatter)
}

fn append(output: &mut String, text: &str) -> Result<(), NoteError> {
    if text.len() > MAX_HEADER_BYTES - output.len() {
        return Err(NoteError::CapacityExceeded);
    }
    output.push_str(text);
    Ok(())
}
