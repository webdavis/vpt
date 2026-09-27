mod visitor;

use serde::de::DeserializeSeed;
use serde_json::Value;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub bytes: usize,
    pub depth: usize,
    pub text_chars: usize,
    pub array_len: usize,
}

impl Limits {
    pub fn incoming() -> Self {
        Self {
            bytes: 65_536,
            depth: 8,
            text_chars: 65_536,
            array_len: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitViolation {
    Bytes(usize),
    Depth(usize),
    Text(usize),
    Array(usize),
    Syntax(String),
}

pub fn read_bounded(bytes: &[u8], limits: &Limits) -> Result<Value, LimitViolation> {
    if bytes.len() > limits.bytes {
        return Err(LimitViolation::Bytes(bytes.len()));
    }
    let context = visitor::Context {
        limits,
        violation: RefCell::new(None),
    };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = visitor::Node {
        context: &context,
        depth: 0,
        overflow: None,
    }
    .deserialize(&mut decoder)
    .and_then(|value| decoder.end().map(|()| value));
    value.map_err(|_| {
        context
            .violation
            .into_inner()
            .unwrap_or_else(|| LimitViolation::Syntax("invalid JSON document".into()))
    })
}

#[cfg(test)]
mod tests;
