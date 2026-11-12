use super::*;

pub(in crate::note) fn plain(text: &str, spans: &[Span<'_>]) -> Result<String, RenderError> {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for span in spans {
        if span.bytes.start < cursor {
            return Err(RenderError::InvalidFlags);
        }
        output.push_str(&text[cursor..span.bytes.start]);
        match span.resolution {
            Resolution::Corrected {
                text: correction, ..
            } => {
                if span.first {
                    let insertion = span.bytes.is_empty();
                    if insertion && output.ends_with(|ch: char| !ch.is_whitespace()) {
                        output.push(' ');
                    }
                    output.push_str(correction);
                    if insertion && text[span.bytes.end..].starts_with(char::is_alphanumeric) {
                        output.push(' ');
                    }
                }
            }
            _ => output.push_str(&text[span.bytes.clone()]),
        }
        cursor = span.bytes.end;
    }
    output.push_str(&text[cursor..]);
    Ok(output)
}
