mod named;
mod toon;
mod yaml;

pub(crate) use named::named;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Format {
    Json,
    Toon,
    Yaml,
}

impl Format {
    pub(crate) fn render(self, document: &Value) -> String {
        match self {
            Format::Json => format!("{document}\n"),
            Format::Toon => toon::render(document),
            Format::Yaml => yaml::render(document),
        }
    }
}

#[cfg(test)]
#[path = "format/tests.rs"]
mod tests;
