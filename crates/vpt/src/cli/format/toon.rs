use serde_json::Value;

pub(super) fn render(document: &Value) -> String {
    let mut lines = Vec::new();
    write_value(document, 0, &mut lines);
    format!("{}\n", lines.join("\n"))
}

fn write_value(value: &Value, depth: usize, lines: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                let key = key_text(key);
                match value {
                    Value::Object(_) => {
                        lines.push(format!("{pad}{key}:"));
                        write_value(value, depth + 1, lines);
                    }
                    Value::Array(items) => write_list(&key, items, depth, lines),
                    scalar => lines.push(format!("{pad}{key}: {}", scalar_toon(scalar))),
                }
            }
        }
        Value::Array(items) => write_list("", items, depth, lines),
        scalar => lines.push(format!("{pad}{}", scalar_toon(scalar))),
    }
}

fn write_list(key: &str, items: &[Value], depth: usize, lines: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    let count = items.len();
    if items
        .iter()
        .all(|item| !item.is_object() && !item.is_array())
    {
        let row = items.iter().map(scalar_toon).collect::<Vec<_>>().join(",");
        let gap = if row.is_empty() { "" } else { " " };
        lines.push(format!("{pad}{key}[{count}]:{gap}{row}"));
        return;
    }
    lines.push(format!("{pad}{key}[{count}]:"));
    let inner = "  ".repeat(depth + 1);
    for item in items {
        let mut nested = Vec::new();
        write_value(item, 0, &mut nested);
        if nested.is_empty() {
            lines.push(format!("{inner}-"));
        }
        let rest = if item.is_array() { "" } else { "  " };
        for (which, line) in nested.into_iter().enumerate() {
            match which {
                0 => lines.push(format!("{inner}- {line}")),
                _ => lines.push(format!("{inner}{rest}{line}")),
            }
        }
    }
}

fn scalar_toon(value: &Value) -> String {
    match value {
        Value::String(text) => quoted_when_bare_would_read_as_something_else(text),
        other => other.to_string(),
    }
}

fn quoted_when_bare_would_read_as_something_else(text: &str) -> String {
    let bare = !text.is_empty()
        && text.trim() == text
        && !text.starts_with('-')
        && !matches!(text, "true" | "false" | "null")
        && text.parse::<f64>().is_err()
        && !text.contains([',', ':', '"', '\\', '[', ']', '{', '}', '#'])
        && !text.chars().any(char::is_control);
    match bare {
        true => text.to_string(),
        false => serde_json::Value::String(text.to_string()).to_string(),
    }
}

fn key_text(key: &str) -> String {
    let bare = key.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'));
    match bare {
        true => key.to_string(),
        false => serde_json::Value::String(key.to_string()).to_string(),
    }
}
