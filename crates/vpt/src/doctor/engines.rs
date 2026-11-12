use super::checks::check;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use vpt_application::engines::{EngineKind, EngineSettings};
use vpt_protocol::error::Check;

pub(super) fn inspect(settings: &EngineSettings, helper_available: bool) -> Check {
    let selected: BTreeSet<_> = std::iter::once(&settings.pair)
        .chain(settings.by_language.values())
        .flat_map(|pair| std::iter::once(&pair.main).chain(pair.checker.iter()))
        .collect();
    let mut ok = true;
    let details: Vec<_> = settings
        .definitions
        .iter()
        .map(|(name, engine)| {
            let used = selected.contains(name);
            let (available, locality) = match &engine.kind {
                EngineKind::Apple => (helper_available, "local"),
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
            ok &= !used || available;
            format!(
                "{name}: {}, {}, {locality}",
                if used { "selected" } else { "unused" },
                if available {
                    "available"
                } else {
                    "unavailable"
                },
            )
        })
        .collect();
    check(
        "engines",
        ok,
        format!(
            "{}; availability checks executable files (Apple also requires a compatible helper); selected includes language overrides",
            details.join("; ")
        ),
    )
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
