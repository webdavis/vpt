use super::checks::Context;
use super::report::Row;
use crate::style::Verdict;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use vpt_application::engines::EngineKind;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let settings = &context.settings.engines;
    let selected: BTreeSet<_> = std::iter::once(&settings.pair)
        .chain(settings.by_language.values())
        .flat_map(|pair| std::iter::once(&pair.main).chain(pair.checker.iter()))
        .collect();
    settings
        .definitions
        .iter()
        .map(|(name, engine)| {
            let used = selected.contains(name);
            let (available, locality) = match &engine.kind {
                EngineKind::Apple => (context.helper().verdict == Verdict::Ok, "local"),
                EngineKind::Whisply {
                    command,
                    hf_token: Some(_),
                    ..
                } if used => (available(command), "local, speaker labels on"),
                EngineKind::Whisply {
                    command,
                    hf_token: Some(_),
                    ..
                } => (
                    available(command),
                    "local, token set but whisply is not selected, so no speaker labels",
                ),
                EngineKind::Whisply { command, .. } => (available(command), "local"),
                EngineKind::Command { command, local, .. } => (
                    available(command),
                    if *local {
                        "declared local"
                    } else {
                        "declared remote (audio leaves the machine)"
                    },
                ),
            };
            let detail = format!(
                "{}, {}, {locality}",
                if used { "selected" } else { "unused" },
                if available { "available" } else { "unavailable" },
            );
            if used && !available {
                let fix = if matches!(engine.kind, EngineKind::Apple) {
                    "see `vpt doctor helper`".to_string()
                } else {
                    format!("make its program an executable file, or change `engines.{name}` in the config")
                };
                Row::issue(name.as_str(), detail).advice(fix)
            } else {
                Row::ok(name.as_str(), detail)
            }
        })
        .collect()
}

fn available(command: &[String]) -> bool {
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
