use serde_json::Value;

pub(super) fn render(document: &Value) -> String {
    let mut lines = Vec::new();
    write_block(document, 0, &mut lines);
    format!("{}\n", lines.join("\n"))
}

fn write_block(value: &Value, depth: usize, lines: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            for (key, value) in fields {
                let key = key_text(key);
                match nested_block(value) {
                    true => {
                        lines.push(format!("{pad}{key}:"));
                        write_block(value, depth + 1, lines);
                    }
                    false => lines.push(format!("{pad}{key}: {}", flow(value))),
                }
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for item in items {
                let mut nested = Vec::new();
                write_block(item, 0, &mut nested);
                for (which, line) in nested.into_iter().enumerate() {
                    match which {
                        0 => lines.push(format!("{pad}- {line}")),
                        _ => lines.push(format!("{pad}  {line}")),
                    }
                }
            }
        }
        flat => lines.push(format!("{pad}{}", flow(flat))),
    }
}

fn nested_block(value: &Value) -> bool {
    match value {
        Value::Object(fields) => !fields.is_empty(),
        Value::Array(items) => !items.is_empty(),
        _ => false,
    }
}

fn flow(value: &Value) -> String {
    match value {
        Value::Object(_) => "{}".into(),
        Value::Array(_) => "[]".into(),
        scalar => scalar.to_string(),
    }
}

fn key_text(key: &str) -> String {
    let plain = key.starts_with(|first: char| first.is_ascii_lowercase() || first == '_')
        && key.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        && !matches!(
            key,
            "y" | "n" | "yes" | "no" | "on" | "off" | "true" | "false" | "null"
        );
    match plain {
        true => key.to_string(),
        false => Value::String(key.to_string()).to_string(),
    }
}
