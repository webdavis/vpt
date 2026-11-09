use super::*;
impl Redactor {
    pub fn apply_with_spans(
        &mut self,
        text: &str,
        protected: &[std::ops::Range<usize>],
    ) -> Result<MaskedText, Error> {
        candidates::validate_ranges(text, protected)?;
        validate_text(text)?;
        let tokens = vocabulary::tokens(text)?;
        let mut found = Vec::new();
        self.terms.scan(&tokens, &mut found)?;
        self.names.scan(&tokens, &mut found)?;
        patterns::scan(text, &self.options.classes, &mut found)?;
        let selected = matches::select(found);
        let mut output = String::new();
        let mut pending = Vec::new();
        let mut masks = Vec::new();
        let mut mapped_bytes = self.mapped_bytes;
        let mut cursor = 0;
        for found in selected {
            if protected
                .get(protected.partition_point(|r| r.end <= found.range.start))
                .is_some_and(|r| r.start < found.range.end)
            {
                return Err(Error::Residue {
                    class: found.class,
                    number: None,
                });
            }

            if self.mappings.len() + pending.len() >= MAX_ITEMS {
                return Err(Error::Capacity);
            }
            let number = self.mappings.len() + pending.len() + 1;
            let label = self
                .options
                .mask
                .replace("{class}", found.class.as_str())
                .replace("{n}", &number.to_string());
            let value = &text[found.range.clone()];
            mapped_bytes = mapped_bytes
                .checked_add(value.len() + label.len())
                .ok_or(Error::Capacity)?;
            if mapped_bytes > MAX_TEXT {
                return Err(Error::Capacity);
            }
            append(&mut output, &text[cursor..found.range.start])?;
            let start = output.len();
            append(&mut output, &label)?;
            masks.push(start..output.len());
            cursor = found.range.end;
            pending.push(Mapping {
                class: found.class,
                number,
                label,
                value: value.into(),
            });
        }
        append(&mut output, &text[cursor..])?;
        self.mappings.extend(pending);
        self.mapped_bytes = mapped_bytes;
        Ok(MaskedText {
            text: output,
            masks,
        })
    }
}
