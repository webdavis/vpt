mod toon;
mod yaml;

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Json,
    Toon,
    Yaml,
}

impl Format {
    pub(crate) const FLAGS: [(&'static str, Format); 3] = [
        ("--json", Format::Json),
        ("--toon", Format::Toon),
        ("--yaml", Format::Yaml),
    ];

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
