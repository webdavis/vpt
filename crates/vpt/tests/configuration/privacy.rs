use super::*;

#[test]
fn unsafe_configuration_permissions_refuse_every_command_without_chmod_or_work() {
    for unsafe_parent in [false, true] {
        let sandbox = fixture::configured("configuration-permissions");
        fixture::write_config(
            &sandbox,
            "[context.google]\nclient_secret='Private configuration value'\n",
        );
        let config = sandbox.config_path();
        let path = if unsafe_parent {
            config.parent().unwrap()
        } else {
            &config
        };
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if unsafe_parent { 0o755 } else { 0o644 }),
        )
        .unwrap();
        let before = fixture::snapshot(&sandbox);
        for args in commands(&sandbox) {
            for structured in [false, true] {
                let mut command = sandbox.vpt();
                command.args(&args);
                if structured {
                    command.arg("--json");
                }
                refusal(&mut command, "private", structured);
                assert_eq!(fixture::snapshot(&sandbox), before, "{args:?}");
            }
        }
    }
}
