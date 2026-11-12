use crate::support::Sandbox;
use std::os::unix::fs::PermissionsExt;

pub fn install(sandbox: &Sandbox) -> std::path::PathBuf {
    let capture = sandbox.path().join("report.jsonl");
    let hook = sandbox.path().join("bin/report");
    std::fs::write(
        &hook,
        format!(
            "#!/bin/sh\n/bin/cat >> '{0}'\nprintf '\\n' >> '{0}'\n",
            capture.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    capture
}

pub fn events(capture: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(capture)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
