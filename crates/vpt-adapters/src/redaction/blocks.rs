use super::*;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
mod build;
mod html;
use build::{flush, push};
pub(super) struct Block {
    pub text: String,
    pub protected: Vec<Range<usize>>,
    pub source: Range<usize>,
    pub heading: Option<usize>,
}
pub(super) fn read(source: &str) -> Result<Vec<Block>, Error> {
    read_with_markers(source, &[])
}
pub(super) fn read_with_markers(
    source: &str,
    markers: &[Range<usize>],
) -> Result<Vec<Block>, Error> {
    let mut seen_markers = 0;
    build::validate(source)?;
    let options = Options::ENABLE_WIKILINKS
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_TABLES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    let mut output = Vec::new();
    let mut block: Option<Block> = None;
    let (mut skip, mut depth) = (0usize, 0usize);
    let mut html = html::Hidden::default();
    let mut text_bytes = 0;
    let mut code = false;
    for (index, (event, range)) in Parser::new_ext(source, options)
        .into_offset_iter()
        .enumerate()
    {
        if index >= MAX_ITEMS * 16 {
            return Err(Error::Capacity);
        }
        if matches!(event, Event::Start(_)) {
            depth += 1;
        }
        if matches!(event, Event::End(_)) {
            depth = depth.saturating_sub(1);
        }
        if depth > 128 {
            return Err(Error::Capacity);
        }
        if skip > 0 {
            match event {
                Event::Start(_) => skip += 1,
                Event::End(_) => skip -= 1,
                _ => (),
            }
            continue;
        }
        if let Event::InlineHtml(value) = &event {
            html.accept(value)?;
            continue;
        }
        if html.active() {
            continue;
        }
        if matches!(event, Event::Start(Tag::CodeBlock(_))) {
            code = true;
        }
        if matches!(event, Event::End(TagEnd::CodeBlock)) {
            code = false;
        }
        match event {
            Event::Start(Tag::MetadataBlock(_) | Tag::HtmlBlock) => {
                flush(&mut block, &mut output)?;
                skip = 1;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut block, &mut output)?;
                block = Some(Block {
                    text: String::new(),
                    protected: vec![],
                    source: range,
                    heading: Some(level as usize),
                });
            }
            Event::Start(Tag::Paragraph | Tag::CodeBlock(_) | Tag::Item | Tag::TableRow) => {
                flush(&mut block, &mut output)?;
                block = Some(Block {
                    text: String::new(),
                    protected: vec![],
                    source: range,
                    heading: None,
                });
            }
            Event::End(
                TagEnd::Heading(_)
                | TagEnd::Paragraph
                | TagEnd::CodeBlock
                | TagEnd::Item
                | TagEnd::TableRow,
            ) => {
                if let Some(block) = &mut block {
                    block.source.end = block.source.end.max(range.end);
                }
                flush(&mut block, &mut output)?;
            }
            Event::Text(text) => {
                let text = if code {
                    html_escape::decode_html_entities(&text)
                } else {
                    std::borrow::Cow::Borrowed(text.as_ref())
                };
                push(&mut block, &text, range, &mut text_bytes)?;
            }
            Event::Code(text) => build::code(
                &mut block,
                &text,
                range,
                markers,
                &mut seen_markers,
                &mut text_bytes,
            )?,
            Event::SoftBreak | Event::HardBreak => push(&mut block, "\n", range, &mut text_bytes)?,
            Event::End(TagEnd::TableCell) => push(&mut block, " ", range, &mut text_bytes)?,
            Event::TaskListMarker(done) => push(
                &mut block,
                if done { "[x] " } else { "[ ] " },
                range,
                &mut text_bytes,
            )?,
            _ => (),
        }
    }
    flush(&mut block, &mut output)?;
    if seen_markers != markers.len() {
        return Err(Error::InvalidText);
    }
    Ok(output)
}
