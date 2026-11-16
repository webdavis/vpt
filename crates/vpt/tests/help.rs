mod support;

use support::{Sandbox, run, stderr, stdout};

const DOCS: &str = "https://github.com/webdavis/vpt#command-line";

const COMMANDS: &[&str] = &[
    "run",
    "setup",
    "doctor",
    "test",
    "ingest",
    "import",
    "show",
    "transcribe",
    "review list",
    "review confirm",
    "review correct",
    "review dismiss",
    "confirm term",
    "confirm tag",
    "confirm reject-tag",
    "confirm relation",
    "tag rename",
    "analyze",
    "path",
    "path occasion",
    "note write",
    "verify-note",
    "occasions",
    "list",
    "search",
    "storage",
    "retention run",
    "brief",
    "brief upcoming",
    "redact",
    "symlink deploy",
    "symlink verify",
    "version",
    "completion",
];

fn help(sandbox: &Sandbox, args: &[&str]) -> String {
    let output = run(sandbox.vpt().args(args));
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        stderr(&output)
    );
    assert_eq!(stderr(&output), "", "{args:?}");
    stdout(&output)
}

#[test]
fn bare_vpt_prints_short_help_on_stdout() {
    let sandbox = Sandbox::new("help-bare");
    let text = help(&sandbox, &[]);
    assert!(text.contains("Usage: vpt"), "{text}");
    assert!(text.contains("transcribe"), "{text}");
}

#[test]
fn help_is_available_at_every_depth() {
    let sandbox = Sandbox::new("help-depth");
    for command in COMMANDS {
        let words: Vec<&str> = command.split(' ').collect();
        let usage = format!("Usage: vpt {command}");
        for flag in ["-h", "--help"] {
            let text = help(&sandbox, &[words.as_slice(), &[flag]].concat());
            assert!(text.contains(&usage), "{command} {flag}: {text}");
        }
        let text = help(&sandbox, &[&["help"], words.as_slice()].concat());
        assert!(text.contains(&usage), "help {command}: {text}");
    }
}

#[test]
fn a_command_group_typed_alone_shows_its_help() {
    let sandbox = Sandbox::new("help-groups");
    for group in ["tag", "note", "retention", "symlink", "review", "confirm"] {
        let text = help(&sandbox, &[group]);
        assert!(text.contains(&format!("Usage: vpt {group}")), "{text}");
    }
}

#[test]
fn every_command_help_carries_two_or_three_examples() {
    let sandbox = Sandbox::new("help-examples");
    for command in COMMANDS {
        let words: Vec<&str> = command.split(' ').collect();
        let text = help(&sandbox, &[words.as_slice(), &["--help"]].concat());
        let (_, examples) = text
            .split_once("Examples:\n")
            .unwrap_or_else(|| panic!("{command}: {text}"));
        let lines: Vec<&str> = examples
            .lines()
            .take_while(|line| line.starts_with("  "))
            .collect();
        assert!((2..=3).contains(&lines.len()), "{command}: {text}");
        let invocation = format!("vpt {command}");
        assert!(
            lines.iter().all(|line| line.contains(&invocation)),
            "{command}: {text}"
        );
    }
}

#[test]
fn short_help_is_terse_and_long_help_ends_with_the_docs_link() {
    let sandbox = Sandbox::new("help-short-long");
    let groups = ["tag", "note", "retention", "symlink", "review", "confirm"];
    for command in [""].iter().chain(&groups).chain(COMMANDS) {
        let words: Vec<&str> = command.split(' ').filter(|w| !w.is_empty()).collect();
        let short = help(&sandbox, &[words.as_slice(), &["-h"]].concat());
        assert!(!short.contains("Examples:"), "{command} -h: {short}");
        assert!(!short.contains(DOCS), "{command} -h: {short}");
        let long = help(&sandbox, &[words.as_slice(), &["--help"]].concat());
        assert!(long.trim_end().ends_with(DOCS), "{command} --help: {long}");
    }
}

#[test]
fn version_is_listed_apart_from_the_options_every_command_takes() {
    let sandbox = Sandbox::new("help-version-heading");
    let text = help(&sandbox, &["--help"]);
    let (options, global) = text.split_once("Global options:").expect("global heading");
    assert!(options.contains("--version"), "{text}");
    assert!(!global.contains("--version"), "{text}");
}
