mod key;
use super::shape::{Location, Profile};
use super::{LimitViolation, Limits};
use key::Key;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::cell::RefCell;
use std::fmt;

pub(super) struct Context<'a> {
    pub limits: &'a Limits,
    pub profile: Profile,
    pub violation: RefCell<Option<LimitViolation>>,
}

impl Context<'_> {
    fn reject<E: de::Error>(&self, violation: LimitViolation) -> E {
        *self.violation.borrow_mut() = Some(violation);
        E::custom("document limit exceeded")
    }

    fn text<E: de::Error>(&self, value: &str) -> Result<(), E> {
        let length = value.chars().count();
        if length > self.limits.text_chars {
            return Err(self.reject(LimitViolation::Text(length)));
        }
        Ok(())
    }
}

pub(super) struct Node<'a, 'b> {
    pub context: &'a Context<'b>,
    pub depth: usize,
    pub location: Location,
    pub overflow: Option<usize>,
}

impl<'de> DeserializeSeed<'de> for Node<'_, '_> {
    type Value = Value;

    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        if let Some(length) = self.overflow {
            return Err(self.context.reject(LimitViolation::Array(length)));
        }
        decoder.deserialize_any(self)
    }
}

impl Node<'_, '_> {
    fn enter<E: de::Error>(&self) -> Result<usize, E> {
        let depth = self.depth + 1;
        if depth > self.context.limits.depth {
            return Err(self.context.reject(LimitViolation::Depth(depth)));
        }
        Ok(depth)
    }
}

impl<'de> Visitor<'de> for Node<'_, '_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        self.context.text(value)?;
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        self.context.text(&value)?;
        Ok(Value::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let depth = self.enter()?;
        let mut items = Vec::new();
        loop {
            let overflow = (items.len()
                >= self
                    .location
                    .limit(self.context.profile, self.context.limits.array_len))
            .then_some(items.len().saturating_add(1));
            let next = sequence.next_element_seed(Node {
                context: self.context,
                depth,
                location: self.location.element(),
                overflow,
            })?;
            match next {
                Some(value) => items.push(value),
                None => return Ok(Value::Array(items)),
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Value, A::Error> {
        let depth = self.enter()?;
        let mut fields = Map::new();
        while let Some(key) = object.next_key_seed(Key(self.context))? {
            if fields.contains_key(&key) {
                return Err(de::Error::custom("duplicate object field"));
            }
            let value = object.next_value_seed(Node {
                context: self.context,
                depth,
                location: self.location.field(&key),
                overflow: None,
            })?;
            fields.insert(key, value);
        }
        Ok(Value::Object(fields))
    }
}
