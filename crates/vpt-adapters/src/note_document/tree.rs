use super::*;
use saphyr_parser::{Event, Parser, ScalarStyle};
use std::{borrow::Cow, collections::HashMap};
mod emit;
pub(super) use emit::{quote, string_list};
mod offsets;
mod read;

pub(super) struct Tree<'a> {
    pub nodes: Vec<Node<'a>>,
    pub root: usize,
    anchors: HashMap<usize, usize>,
}
pub(super) struct Node<'a> {
    pub span: Range<usize>,
    pub kind: Kind<'a>,
}
pub(super) enum Kind<'a> {
    Scalar {
        text: Cow<'a, str>,
        style: ScalarStyle,
        tag: Option<String>,
    },
    Alias(usize),
    Sequence(Vec<usize>),
    Mapping(Vec<usize>),
}

impl<'a> Tree<'a> {
    pub fn parse(source: &'a str) -> Result<Self, NoteError> {
        if source.len() > MAX_HEADER_BYTES {
            return Err(NoteError::CapacityExceeded);
        }
        let offsets = offsets::Offsets::new(source);
        let mut tree = Self {
            nodes: Vec::new(),
            root: 0,
            anchors: HashMap::new(),
        };
        let mut stack = Vec::<usize>::new();
        let mut documents = 0;
        for item in Parser::new_from_str(source) {
            let (event, span) = item.map_err(|_| NoteError::Frontmatter)?;
            let bytes = offsets.byte(span.start.index())..offsets.byte(span.end.index());
            if source.get(bytes.clone()).is_none() {
                return Err(NoteError::Frontmatter);
            }
            let (kind, anchor, container) = match event {
                Event::StreamStart | Event::StreamEnd | Event::DocumentEnd => continue,
                Event::DocumentStart(_) => {
                    documents += 1;
                    if documents > 1 {
                        return Err(NoteError::Frontmatter);
                    }
                    continue;
                }
                Event::MappingEnd | Event::SequenceEnd => {
                    let id = stack.pop().ok_or(NoteError::Frontmatter)?;
                    let end = if bytes.is_empty() {
                        match &tree.nodes[id].kind {
                            Kind::Sequence(children) | Kind::Mapping(children) => children
                                .last()
                                .map(|child| tree.nodes[*child].span.end)
                                .unwrap_or(bytes.end),
                            _ => return Err(NoteError::Frontmatter),
                        }
                    } else {
                        offsets::container_end(source, &bytes)?
                    };
                    tree.nodes[id].span.end = end;
                    continue;
                }
                Event::Scalar(text, style, anchor, tag) => {
                    if text.len() > 4 * 65_536 || text.chars().count() > 65_536 {
                        return Err(NoteError::CapacityExceeded);
                    }
                    (
                        Kind::Scalar {
                            text,
                            style,
                            tag: tag.map(|tag| format!("{}{}", tag.handle, tag.suffix)),
                        },
                        anchor,
                        false,
                    )
                }
                Event::Alias(target) => (Kind::Alias(target), 0, false),
                Event::MappingStart(anchor, _) => (Kind::Mapping(Vec::new()), anchor, true),
                Event::SequenceStart(anchor, _) => (Kind::Sequence(Vec::new()), anchor, true),
                Event::Nothing => return Err(NoteError::Frontmatter),
            };
            if tree.nodes.len() == 65_536 || stack.len() == 64 {
                return Err(NoteError::CapacityExceeded);
            }
            let id = tree.nodes.len();
            if let Some(parent) = stack.last() {
                match &mut tree.nodes[*parent].kind {
                    Kind::Mapping(children) | Kind::Sequence(children) => children.push(id),
                    _ => return Err(NoteError::Frontmatter),
                }
            } else if id != 0 {
                return Err(NoteError::Frontmatter);
            }
            tree.nodes.push(Node { kind, span: bytes });
            if anchor != 0 {
                tree.anchors.insert(anchor, id);
            }
            if container {
                stack.push(id);
            }
        }
        if documents != 1 || tree.nodes.is_empty() || !stack.is_empty() {
            return Err(NoteError::Frontmatter);
        }
        Ok(tree)
    }
}
