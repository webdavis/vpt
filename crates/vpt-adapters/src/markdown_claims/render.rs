use super::*;
use vpt_domain::{grounding::Finding, note, review::Resolution};

impl Claims<'_> {
    pub fn annotate_resolved(
        &self,
        annotations: &[Option<Finding>],
        resolutions: &[Resolution],
    ) -> Result<String, ClaimError> {
        if resolutions.len() != self.claims.len() {
            return Err(ClaimError::InvalidAnnotations);
        }
        self.render(annotations, Some(resolutions))
    }

    pub fn annotate(&self, annotations: &[Option<Finding>]) -> Result<String, ClaimError> {
        self.render(annotations, None)
    }

    fn render(
        &self,
        annotations: &[Option<Finding>],
        resolutions: Option<&[Resolution]>,
    ) -> Result<String, ClaimError> {
        if annotations.len() != self.claims.len() || annotations.len() > MAX_ITEMS {
            return Err(ClaimError::InvalidAnnotations);
        }
        let mut size =
            self.source.len() - self.removals.iter().map(|range| range.len()).sum::<usize>();
        let mut payload = 0usize;
        let mut edits: Vec<_> = self
            .removals
            .iter()
            .map(|range| (range.clone(), String::new()))
            .collect();
        for (index, (claim, finding)) in self.claims.iter().zip(annotations).enumerate() {
            if claim.end > self.source.len() || !self.source.is_char_boundary(claim.end) {
                return Err(ClaimError::InvalidAnnotations);
            }
            if let Some(finding) = finding {
                let resolution = resolutions.map_or(&Resolution::Open, |values| &values[index]);
                payload = payload.saturating_add(match (finding, resolution) {
                    (Finding::Unsupported(token), Resolution::Open) => token.len(),
                    (_, Resolution::Corrected { text, .. }) => text.len(),
                    _ => 0,
                });
                if payload > MAX_TEXT {
                    return Err(ClaimError::CapacityExceeded);
                }
                let Some(annotation) = note::resolved_finding_annotation(finding, resolution)
                    .map_err(|error| match error {
                        note::RenderError::CapacityExceeded => ClaimError::CapacityExceeded,
                        _ => ClaimError::InvalidAnnotations,
                    })?
                else {
                    continue;
                };
                size = size.saturating_add(annotation.len()).saturating_add(1);
                if size > note::MAX_RENDERED {
                    return Err(ClaimError::CapacityExceeded);
                }
                edits.push((claim.end..claim.end, format!(" {annotation}")));
            }
        }
        edits.sort_by_key(|(range, _)| (range.start, range.end));
        let mut output = String::new();
        let mut cursor = 0;
        for (range, replacement) in edits {
            if range.start < cursor || range.end > self.source.len() {
                return Err(ClaimError::InvalidAnnotations);
            }
            append(&mut output, &self.source[cursor..range.start])?;
            append(&mut output, &replacement)?;
            cursor = range.end;
        }
        append(&mut output, &self.source[cursor..])?;
        Ok(output)
    }
}
fn append(output: &mut String, text: &str) -> Result<(), ClaimError> {
    if text.len() > note::MAX_RENDERED - output.len() {
        return Err(ClaimError::CapacityExceeded);
    }
    output.push_str(text);
    Ok(())
}
