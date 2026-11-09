use super::Context;
use serde::de::{self, DeserializeSeed, Visitor};
use std::fmt;

pub(super) struct Key<'a, 'b>(pub(super) &'a Context<'b>);

impl<'de> DeserializeSeed<'de> for Key<'_, '_> {
    type Value = String;

    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<String, D::Error> {
        decoder.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for Key<'_, '_> {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded object key")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
        self.0.text(value)?;
        Ok(value.to_owned())
    }
}
