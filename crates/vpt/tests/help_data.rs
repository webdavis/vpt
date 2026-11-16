mod support;

use serde_json::{Value, json};
use std::collections::BTreeSet;
use support::{Sandbox, run, stderr, stdout};

fn vpt(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    let config = sandbox.config_path();
    let config = config.to_str().expect("utf-8 path");
    run(sandbox.vpt().args(["--config", config]).args(args))
}

fn document(sandbox: &Sandbox, args: &[&str]) -> Value {
    let output = vpt(sandbox, args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        stderr(&output)
    );
    assert_eq!(stderr(&output), "", "{args:?}");
    serde_json::from_str(&stdout(&output)).expect("json document")
}

fn human(sandbox: &Sandbox, args: &[&str]) -> String {
    stdout(&vpt(sandbox, args))
}

fn listed_commands(help: &str) -> BTreeSet<String> {
    let (_, commands) = help.split_once("Commands:\n").expect("commands heading");
    commands
        .lines()
        .take_while(|line| line.starts_with("  "))
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .collect()
}

#[test]
fn the_root_document_lists_every_command_the_human_help_lists() {
    let sandbox = Sandbox::new("help-data-root");
    let root = document(&sandbox, &["help", "-o", "json"]);
    assert_eq!(root["schema"], "vpt.result/1");
    assert_eq!(root["command"], "help");
    assert_eq!(root["path"], json!(["vpt"]));
    let listed: BTreeSet<String> = root["subcommands"]
        .as_array()
        .expect("subcommands")
        .iter()
        .map(|command| command["name"].as_str().expect("name").to_owned())
        .collect();
    assert_eq!(listed, listed_commands(&human(&sandbox, &["--help"])));
}

#[test]
fn a_command_document_uses_the_words_of_its_human_help() {
    let sandbox = Sandbox::new("help-data-confirm");
    let confirm = document(&sandbox, &["help", "review", "confirm", "-o", "json"]);
    let text = human(&sandbox, &["review", "confirm", "--help"]);
    assert_eq!(confirm["name"], "confirm");
    assert_eq!(confirm["path"], json!(["vpt", "review", "confirm"]));
    assert!(
        text.starts_with(confirm["about"].as_str().expect("about")),
        "{text}"
    );
    assert!(text.contains(&format!(
        "Usage: {}",
        confirm["usage"].as_str().expect("usage")
    )));
    let arguments = confirm["arguments"].as_array().expect("arguments");
    assert_eq!(arguments.len(), 2, "{confirm}");
    for argument in arguments {
        assert_eq!(argument["required"], true, "{argument}");
        assert!(text.contains(&format!("<{}>", argument["name"].as_str().expect("name"))));
        assert!(
            text.contains(argument["help"].as_str().expect("help")),
            "{argument}"
        );
    }
    let output = confirm["options"]
        .as_array()
        .expect("options")
        .iter()
        .find(|option| option["long"] == "--output")
        .expect("--output");
    assert_eq!(output["short"], "-o");
    assert_eq!(output["value_name"], "FORMAT");
    assert_eq!(output["values"], json!(["json", "toon", "yaml"]));
    assert_eq!(output["global"], true);
    assert!(text.contains(output["help"].as_str().expect("help")));
}

#[test]
fn examples_equal_the_human_ones() {
    let sandbox = Sandbox::new("help-data-examples");
    for words in [
        &["review", "confirm"][..],
        &["analyze"],
        &["actions", "resend"],
    ] {
        let data = document(&sandbox, &[&["help"], words, &["-o", "json"]].concat());
        let text = human(&sandbox, &[words, &["--help"]].concat());
        let (_, examples) = text.split_once("Examples:\n").expect("examples");
        let lines: Vec<Value> = examples
            .lines()
            .take_while(|line| line.starts_with("  "))
            .map(|line| Value::from(line.trim()))
            .collect();
        assert_eq!(data["examples"], Value::from(lines), "{words:?}");
    }
}

#[test]
fn toon_and_yaml_render_the_same_document() {
    let sandbox = Sandbox::new("help-data-formats");
    let json = document(&sandbox, &["help", "search", "-o", "json"]);
    let toon = human(&sandbox, &["help", "search", "-o", "toon"]);
    let yaml = human(&sandbox, &["help", "search", "-o", "yaml"]);
    assert_eq!(
        toon_format::decode_default::<Value>(&toon).expect("toon"),
        json
    );
    assert_eq!(serde_yaml_ng::from_str::<Value>(&yaml).expect("yaml"), json);
}

#[test]
fn help_without_a_format_prints_the_long_help() {
    let sandbox = Sandbox::new("help-data-human");
    for words in [&[][..], &["review"], &["review", "confirm"]] {
        assert_eq!(
            human(&sandbox, &[&["help"], words].concat()),
            human(&sandbox, &[words, &["--help"]].concat()),
            "{words:?}"
        );
    }
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    let sandbox = Sandbox::new("help-data-unknown");
    let output = vpt(&sandbox, &["help", "review", "nonsense", "-o", "json"]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert!(!stderr(&output).contains("nonsense"), "{}", stderr(&output));
    let error: Value = serde_json::from_str(&stderr(&output)).expect("error document");
    assert_eq!(error["error"]["kind"], "usage", "{error}");
}
