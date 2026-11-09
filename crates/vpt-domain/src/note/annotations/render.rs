use super::*;

pub(in crate::note) fn render(
    output: &mut Writer,
    text: &str,
    spans: &[Span<'_>],
    enabled: bool,
) -> Result<Vec<super::super::OpenSpan>, RenderError> {
    let ranges: Vec<_> = spans.iter().map(|span| span.bytes.clone()).collect();
    let readable = readability::apply(text, &ranges, enabled).map_err(|error| match error {
        readability::ReadabilityError::CapacityExceeded => RenderError::CapacityExceeded,
        readability::ReadabilityError::InvalidRanges => RenderError::InvalidFlags,
    })?;
    let mut cursor = 0;
    let mut open_spans = Vec::new();
    for (span, range) in spans.iter().zip(&readable.protected) {
        output.text(&readable.text[cursor..range.start])?;
        let lexical = span.lexical;
        let marker = matches!(span.resolution, Resolution::Corrected { .. })
            || (*span.resolution == Resolution::Open && lexical.class.surfaced());
        let insertion = marker && range.is_empty();
        if insertion && readable.text[..range.start].ends_with(|ch: char| !ch.is_whitespace()) {
            output.literal(" ")?;
        }
        let start = output.len();
        let text = &readable.text[range.clone()];
        match span.resolution {
            Resolution::Corrected { text, .. } => {
                if span.first {
                    output.literal("[corrected: ")?;
                    output.text(text)?;
                    output.literal("]")?;
                }
            }
            Resolution::Open if lexical.class.surfaced() => {
                output.literal("[unverified: ")?;
                output.text(text)?;
                if span.first
                    && let Some(alternative) = &lexical.alternative_text
                {
                    output.literal(" | ")?;
                    output.text(alternative)?;
                }
                output.literal("]")?;
            }
            _ => output.text(text)?,
        }
        if *span.resolution == Resolution::Open {
            open_spans.push(super::super::OpenSpan {
                flag: span.flag,
                occurrence: span.occurrence,
                bytes: start..output.len(),
                accepted: text.into(),
            });
        }
        if insertion && readable.text[range.end..].starts_with(char::is_alphanumeric) {
            output.literal(" ")?;
        }
        cursor = range.end;
    }
    output.text(&readable.text[cursor..])?;
    Ok(open_spans)
}
