use super::*;

impl Tree<'_> {
    pub(super) fn resolve(&self, mut id: usize) -> Result<&Node<'_>, NoteError> {
        for _ in 0..64 {
            let node = &self.nodes[id];
            if let Kind::Alias(target) = node.kind {
                id = *self.anchors.get(&target).ok_or(NoteError::Frontmatter)?;
            } else {
                return Ok(node);
            }
        }
        Err(NoteError::CapacityExceeded)
    }

    pub fn string(&self, id: usize) -> Result<&str, NoteError> {
        match &self.resolve(id)?.kind {
            Kind::Scalar { text, tag, .. }
                if tag
                    .as_deref()
                    .is_none_or(|tag| tag == "tag:yaml.org,2002:str") =>
            {
                Ok(text)
            }
            _ => Err(NoteError::Frontmatter),
        }
    }

    pub fn is_null(&self, id: usize) -> Result<bool, NoteError> {
        Ok(matches!(
            &self.resolve(id)?.kind,
            Kind::Scalar { text, style, tag }
                if matches!(text.as_ref(), "" | "~" | "null" | "Null" | "NULL")
                    && (tag.as_deref() == Some("tag:yaml.org,2002:null")
                        || tag.is_none() && *style == ScalarStyle::Plain)
        ))
    }

    pub fn integer(&self, id: usize) -> Result<u32, NoteError> {
        let Kind::Scalar { text, style, tag } = &self.resolve(id)?.kind else {
            return Err(NoteError::Frontmatter);
        };
        if tag.as_deref() != Some("tag:yaml.org,2002:int")
            && (tag.is_some() || *style != ScalarStyle::Plain)
        {
            return Err(NoteError::Frontmatter);
        }
        let text = text.strip_prefix('+').unwrap_or(text);
        let (digits, radix) = if let Some(digits) = text.strip_prefix("0x") {
            (digits, 16)
        } else if let Some(digits) = text.strip_prefix("0o") {
            (digits, 8)
        } else {
            (text, 10)
        };
        u32::from_str_radix(digits, radix).map_err(|_| NoteError::Frontmatter)
    }

    pub fn strings(&self, id: usize) -> Result<Vec<String>, NoteError> {
        let Kind::Sequence(children) = &self.resolve(id)?.kind else {
            return Err(NoteError::Frontmatter);
        };
        if children.len() > 256 {
            return Err(NoteError::CapacityExceeded);
        }
        let mut remaining = MAX_HEADER_BYTES;
        children
            .iter()
            .map(|id| {
                let text = self.string(*id)?;
                remaining = remaining
                    .checked_sub(text.len())
                    .ok_or(NoteError::CapacityExceeded)?;
                Ok(text.to_owned())
            })
            .collect()
    }
}
