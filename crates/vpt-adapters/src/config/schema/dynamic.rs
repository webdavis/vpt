use super::{Kind, keys};

pub struct DynamicTable {
    pub prefix: &'static str,
    pub leaves: &'static [(&'static str, Kind)],
}

pub const DYNAMIC_TABLES: &[DynamicTable] = &[
    DynamicTable {
        prefix: "engines.by_language.",
        leaves: &[("main", Kind::Text), ("checker", Kind::Text)],
    },
    DynamicTable {
        prefix: "engines.",
        leaves: &[
            ("kind", Kind::Enum(&["apple", "whisply", "command"])),
            ("command", Kind::Argv),
            ("family", Kind::Text),
            ("local", Kind::Bool),
            ("device", Kind::Text),
            ("model", Kind::Text),
            ("hf_token", Kind::Secret),
        ],
    },
];

pub fn spec_for(path: &str) -> Option<Kind> {
    if let Some(known) = keys().find(|key| key.path == path) {
        return Some(known.kind);
    }
    for table in DYNAMIC_TABLES {
        if let Some(rest) = path.strip_prefix(table.prefix) {
            let mut parts = rest.splitn(2, '.');
            let (Some(name), Some(leaf)) = (parts.next(), parts.next()) else {
                return None;
            };
            if name.is_empty() || leaf.contains('.') {
                return None;
            }
            if let Some((_, kind)) = table.leaves.iter().find(|(known, _)| *known == leaf) {
                return Some(*kind);
            }
            return None;
        }
    }
    None
}

pub(crate) fn table_is_declared(path: &str) -> bool {
    let as_prefix = format!("{path}.");
    keys().any(|key| key.path.starts_with(&as_prefix))
        || DYNAMIC_TABLES.iter().any(|table| {
            table.prefix.starts_with(&as_prefix)
                || path
                    .strip_prefix(table.prefix)
                    .is_some_and(|name| !name.is_empty() && !name.contains('.'))
        })
}
