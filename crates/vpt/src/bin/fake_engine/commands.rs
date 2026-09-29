use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::path::Path;

pub(super) fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("--version") => version(args),
        Some("trash") => trash(args.get(1)),
        Some("command-sink") => command_sink(&args[1..]),
        Some("process-exchange") => super::exchange::run(&args[1..]),
        _ => 2,
    }
}

fn version(args: &[String]) -> i32 {
    if args.len() != 1 {
        return 91;
    }
    let mode = std::env::var("VPT_FAKE_VERSION_MODE").unwrap_or_default();
    let version = std::env::var("VPT_FAKE_VERSION").unwrap_or_else(|_| "1.0.0".into());
    match mode.as_str() {
        "" => emit(json!({"schema": "vpt.helper/1", "version": version})),
        "additive" => {
            emit(json!({"schema": "vpt.helper/1", "version": version, "capabilities": []}))
        }
        "private-failure" => private_failure(),
        _ => 2,
    }
}

fn private_failure() -> i32 {
    let mut output = io::stdout().lock();
    if output
        .write_all(b"private-stdout-canary")
        .and_then(|()| output.flush())
        .is_err()
        || io::stderr().write_all(b"private-stderr-canary").is_err()
    {
        1
    } else {
        7
    }
}

fn emit(document: Value) -> i32 {
    let mut output = io::stdout().lock();
    if serde_json::to_writer(&mut output, &document).is_err() || writeln!(output).is_err() {
        1
    } else {
        0
    }
}

fn log(document: Value) -> io::Result<()> {
    let Some(path) = std::env::var_os("VPT_FAKE_LOG") else {
        return Ok(());
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    serde_json::to_writer(&mut file, &document)?;
    writeln!(file)
}

fn trash(path: Option<&String>) -> i32 {
    let Some(path) = path else {
        return 2;
    };
    let Some(destination) = std::env::var_os("VPT_FAKE_TRASH") else {
        return 1;
    };
    let Some(name) = Path::new(path).file_name() else {
        return 1;
    };
    if std::fs::rename(path, Path::new(&destination).join(name)).is_err() {
        return 1;
    }
    if log(json!({"command": "trash", "path": path})).is_err() {
        return 1;
    }
    emit(json!({"trashed": path}))
}

fn command_sink(args: &[String]) -> i32 {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err()
        || log(json!({"command": "command-sink", "argv": args, "stdin": input})).is_err()
    {
        return 1;
    }
    std::env::var("VPT_FAKE_EXIT")
        .ok()
        .and_then(|exit| exit.parse().ok())
        .unwrap_or(0)
}
