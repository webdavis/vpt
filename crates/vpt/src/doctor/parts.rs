use crate::report::Row;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub(super) fn not_tested(part: &str) -> Row {
    Row::not_run("test", "not tested").advice(format!("run `vpt test {part}`"))
}

pub(super) fn not_configured(name: &str, key: &str) -> Row {
    Row::not_run(name, format!("not configured; set `{key}` in the config"))
}

pub(super) fn command(name: &str, command: &[String], key: &str, reach: &str) -> Row {
    if available(command) {
        Row::ok(name, format!("configured, available; {reach}"))
    } else {
        Row::issue(name, format!("configured, unavailable; {reach}")).advice(fix(key))
    }
}

pub(super) fn fix(key: &str) -> String {
    format!("make its program an executable file, or change `{key}` in the config")
}

pub(super) fn available(command: &[String]) -> bool {
    let Some(program) = command.first() else {
        return false;
    };
    if program.contains('/') {
        return executable(Path::new(program));
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|path| executable(&path.join(program)))
    })
}

fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
