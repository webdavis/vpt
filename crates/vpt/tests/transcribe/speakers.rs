use super::*;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const TOKEN: &str = "made-up-speaker-fixture-not-a-credential";

pub(super) struct Whisply {
    pub args: PathBuf,
    pub token: PathBuf,
}

pub(super) fn whisply(sandbox: &Sandbox, words: &[Value], config: &str, token: bool) -> Whisply {
    let script = sandbox.path().join("whisply.sh");
    let source = sandbox.path().join("whisply.json");
    let paths = Whisply {
        args: sandbox.path().join("whisply.args"),
        token: sandbox.path().join("whisply.token"),
    };
    let text: Vec<_> = words.iter().map(|word| word["word"].clone()).collect();
    let text = text.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    let document = json!({"device":"mlx","model":"large-v3-turbo","transcription":{"fr":{
        "text":text.join(" "),"chunks":[{"timestamp":[words[0]["start"], words[words.len() - 1]["end"]],
        "text":text.join(" "),"words":words}]}}});
    fs::write(&source, document.to_string()).unwrap();
    fs::write(&script, "#!/bin/sh\nset -eu\nsource=$1\nargs=$2\ntoken=$3\nshift 3\nprintf '%s\\n' \"$@\" > \"$args\"\nprintf '%s' \"${HF_TOKEN-unset}\" > \"$token\"\n[ -f \"$args.exit\" ] && exit \"$(/bin/cat \"$args.exit\")\"\n/bin/mkdir -p \"$5/audio\"\n/bin/cat \"$source\" > \"$5/audio/audio.json\"\n").unwrap();
    fixture::write_config(
        sandbox,
        &format!(
            "[recording]\nlanguage='fr-CA'\n{config}\n[engines.whisply]\nkind='whisply'\ncommand=['/bin/sh', {script:?}, {source:?}, {:?}, {:?}]\n{}",
            paths.args,
            paths.token,
            if token {
                format!("hf_token = {TOKEN:?}\n")
            } else {
                String::new()
            }
        ),
    );
    paths
}

fn files(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, found);
        } else {
            found.push(path);
        }
    }
}

pub(super) fn assert_no_leak(sandbox: &Sandbox, outputs: &[&std::process::Output]) {
    let mut found = Vec::new();
    files(sandbox.path(), &mut found);
    let needle = TOKEN.as_bytes();
    for path in found {
        if path == sandbox.config_path() || path == sandbox.path().join("whisply.token") {
            continue;
        }
        let bytes = fs::read(&path).unwrap_or_default();
        assert!(
            !bytes.windows(needle.len()).any(|window| window == needle),
            "token leaked into {}",
            path.display()
        );
    }
    for output in outputs {
        assert!(!stdout(output).contains(TOKEN));
        assert!(!stderr(output).contains(TOKEN));
    }
}

#[test]
fn a_configured_token_asks_whisply_for_speakers_through_the_environment_only() {
    let (sandbox, row) = configured();
    let words = [json!({"word":"bonjour","start":0.0,"end":1.5})];
    let paths = whisply(&sandbox, &words, "[engines]\nmain='whisply'", true);
    let output = run(&sandbox, row.id.as_str(), &[]);
    json(&output);
    let argv = fs::read_to_string(&paths.args).unwrap();
    let argv: Vec<_> = argv.lines().collect();
    assert_eq!(
        &argv[5..],
        [
            "-d",
            "mlx",
            "-m",
            "large-v3-turbo",
            "-l",
            "fr",
            "-e",
            "json",
            "-a"
        ]
    );
    assert_eq!(fs::read_to_string(&paths.token).unwrap(), TOKEN);
    assert_no_leak(&sandbox, &[&output]);
}

#[test]
fn without_a_token_whisply_runs_as_before() {
    let (sandbox, row) = configured();
    let words = [json!({"word":"bonjour","start":0.0,"end":1.5})];
    let paths = whisply(&sandbox, &words, "[engines]\nmain='whisply'", false);
    json(&run(&sandbox, row.id.as_str(), &[]));
    let argv = fs::read_to_string(&paths.args).unwrap();
    assert_eq!(argv.lines().count(), 13);
    assert!(!argv.lines().any(|argument| argument == "-a"));
    assert_eq!(fs::read_to_string(&paths.token).unwrap(), "unset");
}

#[test]
fn a_whisply_without_speaker_annotation_is_named_in_the_failure() {
    let (sandbox, row) = configured();
    let words = [json!({"word":"bonjour","start":0.0,"end":1.5})];
    let paths = whisply(&sandbox, &words, "[engines]\nmain='whisply'", true);
    fs::write(paths.args.with_extension("args.exit"), "2").unwrap();
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: Value = serde_json::from_str(&stderr(&output)).unwrap();
    let detail = error["error"]["diagnostics"].to_string();
    assert!(detail.contains("speaker annotation"), "{detail}");
    assert_no_leak(&sandbox, &[&output]);
}
