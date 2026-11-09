use super::*;

pub(super) fn push(
    block: &mut Option<Block>,
    text: &str,
    range: Range<usize>,
    bytes: &mut usize,
) -> Result<(), Error> {
    *bytes = bytes.checked_add(text.len()).ok_or(Error::Capacity)?;
    if *bytes > MAX_TEXT {
        return Err(Error::Capacity);
    }
    let block = block.get_or_insert_with(|| Block {
        text: String::new(),
        protected: vec![],
        source: range.clone(),
        heading: None,
    });
    block.text.push_str(text);
    block.source.end = block.source.end.max(range.end);
    Ok(())
}
pub(super) fn flush(block: &mut Option<Block>, out: &mut Vec<Block>) -> Result<(), Error> {
    if let Some(block) = block.take().filter(|b| !b.text.trim().is_empty()) {
        if out.len() >= MAX_ITEMS {
            return Err(Error::Capacity);
        }
        out.push(block);
    }
    Ok(())
}

pub(super) fn code(
    block: &mut Option<Block>,
    text: &str,
    range: Range<usize>,
    markers: &[Range<usize>],
    seen: &mut usize,
    bytes: &mut usize,
) -> Result<(), Error> {
    let marker = markers
        .get(markers.partition_point(|r| r.end <= range.start))
        .is_some_and(|r| r.start < range.end);
    if marker && text != "[unverified]" {
        return Err(Error::InvalidText);
    }
    let start = block.as_ref().map_or(0, |b| b.text.len());
    push(
        block,
        &html_escape::decode_html_entities(text),
        range,
        bytes,
    )?;
    if marker {
        let block = block.as_mut().ok_or(Error::InvalidText)?;
        block.protected.push(start..block.text.len());
        *seen += 1;
    }
    Ok(())
}

pub(super) fn validate(source: &str) -> Result<(), Error> {
    if source.len() > vpt_domain::note::MAX_RENDERED || source.contains('\0') {
        return Err(if source.len() > vpt_domain::note::MAX_RENDERED {
            Error::Capacity
        } else {
            Error::InvalidText
        });
    }
    if source
        .bytes()
        .filter(|c| c.is_ascii_punctuation())
        .take(MAX_ITEMS * 16 + 1)
        .count()
        > MAX_ITEMS * 16
    {
        return Err(Error::Capacity);
    }
    Ok(())
}
