use super::*;
use std::{path::Path, time::Duration};
use vpt_domain::layout::StoreKey;

fn invoke_selected(
    sandbox: &Sandbox,
    selected: &Path,
    explicit: bool,
    args: &[&str],
) -> std::process::Output {
    let mut command = sandbox.vpt();
    command.args(args).arg("--json");
    if explicit {
        command.arg("--config").arg(selected);
    } else {
        command.env("VPT_CONFIG", selected);
    }
    support::run_with_budget(
        &mut command,
        Duration::from_secs(if args[0] == "run" { 8 } else { 5 }),
    )
}

#[test]
fn configuration_overrides_publish_every_artifact_to_its_independent_store() {
    for explicit in [false, true] {
        let sandbox = configured();
        analysis::enable(&sandbox);
        analysis::append(&sandbox, "\n[share]\nautomatic=true\n");
        let source = add(&sandbox, "one.m4a", 1_787_604_456);
        let source_before: Vec<_> = fixture::snapshot(&sandbox)
            .into_iter()
            .filter(|entry| entry.is_within(&source))
            .collect();
        let roots: Vec<_> = StoreKey::all()
            .into_iter()
            .enumerate()
            .map(|(index, key)| {
                let parent = sandbox.path().join(format!("external-{index}"));
                fs::create_dir(&parent).unwrap();
                fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
                (key, parent.join(format!("chosen-{}", key.key_name())))
            })
            .collect();
        let mut stores = String::from("\n[stores]\n");
        for (key, root) in &roots {
            stores.push_str(&format!("{}={root:?}\n", key.key_name()));
        }
        analysis::append(&sandbox, &stores);
        let selected = sandbox.config_path().with_file_name("selected.toml");
        fs::copy(sandbox.config_path(), &selected).unwrap();
        fs::set_permissions(&selected, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(
            sandbox.config_path(),
            "config_version=1\ndefault_must_not_load=true\n",
        )
        .unwrap();

        let result = json(&invoke_selected(&sandbox, &selected, explicit, &["run"]));
        assert_eq!(result["redacted"].as_array().unwrap().len(), 2);
        let row = ledger(&sandbox).recordings().unwrap().remove(0);
        assert_eq!(row.stages.analysis, StageState::Succeeded);
        let brief = json(&invoke_selected(
            &sandbox,
            &selected,
            explicit,
            &[
                "brief",
                "--title",
                "Independent stores",
                "--at",
                "2026-10-01T10:00:00-06:00",
            ],
        ));
        assert_eq!(brief["schema"], "vpt.brief/1");
        let ledger = ledger(&sandbox);
        let artifacts = ledger.retention_artifacts().unwrap();
        for (key, root) in &roots {
            assert!(
                artifacts
                    .iter()
                    .any(|artifact| artifact.path.parent() == Some(root.as_path())),
                "{key:?}"
            );
            assert!(fs::read_dir(root).unwrap().next().is_some(), "{key:?}");
            if key.is_private() {
                assert_eq!(
                    fs::metadata(root).unwrap().permissions().mode() & 0o777,
                    0o700
                );
            }
            assert_eq!(
                fs::metadata(root.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
            assert!(
                !sandbox
                    .path()
                    .join("home/.vpt")
                    .join(key.default_leaf())
                    .exists()
            );
        }
        for artifact in &artifacts {
            assert!(
                roots
                    .iter()
                    .any(|(_, root)| artifact.path.parent() == Some(root.as_path()))
            );
            assert!(artifact.path.is_file());
        }
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(analysis::calls(&sandbox, "calls"), 1);
        assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
        let source_after: Vec<_> = fixture::snapshot(&sandbox)
            .into_iter()
            .filter(|entry| entry.is_within(&source))
            .collect();
        assert_eq!(source_after, source_before);
    }
}
