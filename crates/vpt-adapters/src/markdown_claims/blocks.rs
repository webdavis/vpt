use super::{ClaimError, MAX_ITEMS, MAX_TEXT};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;
mod mapped;
pub(super) use mapped::Block;

pub(super) fn read(source: &str, frontmatter: bool) -> Result<Vec<Block>, ClaimError> {
    let mut options = Options::ENABLE_TASKLISTS | Options::ENABLE_WIKILINKS;
    if frontmatter {
        options |= Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
            | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    }
    let mut blocks: Vec<Block> = Vec::new();
    let mut items = Vec::new();
    let mut paragraph = None;
    let (mut skip, mut links, mut literal, mut depth) = (0usize, false, 0usize, 0usize);
    let (mut bytes, mut pieces) = (0usize, 0usize);
    for (index, (event, range)) in Parser::new_ext(source, options)
        .into_offset_iter()
        .enumerate()
    {
        if index >= MAX_ITEMS * 16 {
            return Err(ClaimError::CapacityExceeded);
        }
        if let Event::Html(text) = &event {
            if text.trim() == vpt_domain::note::managed::LINKS_START {
                links = true;
            }
            if text.trim() == vpt_domain::note::managed::LINKS_END {
                links = false;
            }
        }
        if matches!(event, Event::Start(_)) {
            depth += 1;
        }
        if matches!(event, Event::End(_)) {
            depth = depth.saturating_sub(1);
        }
        if depth > 128 {
            return Err(ClaimError::CapacityExceeded);
        }
        if links {
            continue;
        }
        if skip > 0 {
            match event {
                Event::Start(_) => skip += 1,
                Event::End(_) => skip -= 1,
                _ => (),
            }
            continue;
        }
        let code = matches!(&event, Event::Code(_));
        match event {
            Event::Start(Tag::Heading { .. } | Tag::CodeBlock(_) | Tag::MetadataBlock(_)) => {
                skip = 1
            }
            Event::Start(Tag::Item) => {
                items.push(add(&mut blocks, range.start, true)?);
                paragraph = None;
            }
            Event::End(TagEnd::Item) => {
                items.pop();
                paragraph = None;
            }
            Event::Start(Tag::Paragraph) => {
                if items.is_empty() {
                    paragraph = Some(add(&mut blocks, range.start, false)?);
                } else if let Some(&item) = items.last()
                    && !blocks[item].text.is_empty()
                {
                    blocks[item].push(source, "\n\n", range.start..range.start, false)?;
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(block) = current(&mut blocks, &items, paragraph) {
                    block.end = trimmed_end(source, range.end);
                }
                paragraph = None;
            }
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => literal += 1,
            Event::End(TagEnd::Link | TagEnd::Image) => {
                literal = literal.saturating_sub(1);
                if let Some(block) = current(&mut blocks, &items, paragraph) {
                    block.end = range.end;
                }
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                if let Some(block) = current(&mut blocks, &items, paragraph) {
                    block.end = range.end;
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(block) = current(&mut blocks, &items, paragraph) {
                    bytes = bytes.saturating_add(text.len());
                    pieces += 1;
                    if bytes > MAX_TEXT || pieces > MAX_ITEMS * 4 {
                        return Err(ClaimError::CapacityExceeded);
                    }
                    let plain = literal == 0 && !code;
                    block.push(source, &text, range, plain)?;
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(block) = current(&mut blocks, &items, paragraph) {
                    block.push(source, "\n", range, false)?;
                }
            }
            _ => (),
        }
    }
    Ok(blocks)
}

fn add(blocks: &mut Vec<Block>, start: usize, list: bool) -> Result<usize, ClaimError> {
    if blocks.len() >= MAX_ITEMS {
        return Err(ClaimError::CapacityExceeded);
    }
    blocks.push(Block::new(start, list));
    Ok(blocks.len() - 1)
}
fn current<'a>(
    blocks: &'a mut [Block],
    items: &[usize],
    paragraph: Option<usize>,
) -> Option<&'a mut Block> {
    items
        .last()
        .copied()
        .or(paragraph)
        .map(|index| &mut blocks[index])
}
fn trimmed_end(source: &str, end: usize) -> usize {
    source[..end]
        .trim_end_matches([' ', '\t', '\r', '\n'])
        .len()
}
