use super::schema::{KEYS, Kind};

pub fn template(main_engine: &str, state_dir: &str) -> String {
    render(Some((main_engine, state_dir)))
}

pub(super) fn render(overrides: Option<(&str, &str)>) -> String {
    let mut text = String::new();
    let mut current_table = String::new();
    for key in KEYS {
        let (table, leaf) = split_table(key.path);
        if table != current_table {
            if !table.is_empty() {
                text.push_str(&format!("\n[{table}]\n"));
            }
            current_table = table.to_owned();
        }
        let value = match (key.path, overrides) {
            ("engines.main", Some((main_engine, _))) => quoted(main_engine),
            ("home.state_dir", Some((_, state_dir))) => quoted(state_dir),
            _ => key.default.to_owned(),
        };
        let secret = if key.kind == Kind::Secret {
            " (secret)"
        } else {
            ""
        };
        text.push_str(&format!("{leaf} = {value} # {}{secret}\n", key.comment));
    }
    text
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value)
        .expect("strings serialize")
        .replace('\u{7f}', "\\u007f")
}

fn split_table(path: &str) -> (&str, &str) {
    match path.rfind('.') {
        Some(index) => (&path[..index], &path[index + 1..]),
        None => ("", path),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_starts_with_the_version_and_names_the_chosen_engine() {
        let text = template("whisply", "/state/vpt");
        assert!(text.starts_with("config_version = 1"), "{text}");
        assert!(text.contains("\nmain = \"whisply\""), "{text}");
        assert!(text.contains("state_dir = \"/state/vpt\""), "{text}");
    }

    #[test]
    fn every_key_line_carries_a_comment() {
        for line in template("apple", "~/.local/state/vpt").lines() {
            if line.contains(" = ") {
                assert!(line.contains(" # "), "uncommented key line: {line}");
            }
        }
    }

    #[test]
    fn a_secret_key_is_marked_in_its_comment() {
        let text = template("apple", "~/.local/state/vpt");
        assert!(
            text.contains("client_secret = \"\" # OAuth client secret (secret)"),
            "{text}"
        );
    }

    #[test]
    fn all_control_characters_and_unicode_round_trip_as_values() {
        let value: String = (0..=127).map(char::from).chain("雪é".chars()).collect();
        let text = template(&value, &value);
        let table: toml::Table = text.parse().expect("escaped template parses");
        assert_eq!(
            super::super::get(&table, "home.state_dir").and_then(toml::Value::as_str),
            Some(value.as_str())
        );
        assert_eq!(
            super::super::get(&table, "engines.main").and_then(toml::Value::as_str),
            Some(value.as_str())
        );
    }
}
