use super::*;

const VALUE: &str = "made-up-action-env-fixture-not-a-credential";

fn configure(sandbox: &Sandbox, env: &str) -> std::path::PathBuf {
    let script = sandbox.path().join("todo-tool.sh");
    let seen = sandbox.path().join("seen");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nif [ \"${{TODO_TOOL_TOKEN-unset}}\" = {VALUE:?} ]; then token=match; else token=\"${{TODO_TOOL_TOKEN-unset}}\"; fi\nprintf '%s %s %s\\n' \"$token\" \"${{HOME:+home}}\" \"$1\" >> {seen:?}\n"
        ),
    )
    .unwrap();
    agent(
        sandbox,
        &format!("[actions]\ncommand = ['/bin/sh', {script:?}, '{{id}}']\n{env}"),
    );
    crate::actions::propose(
        sandbox,
        r#"[{"text":"call the bank","sources":["00:00-00:01"],"owner":"me"}]"#,
    );
    seen
}

#[test]
fn the_action_command_gets_the_configured_variables_on_top_of_the_inherited_environment() {
    let (sandbox, record) = note::configured();
    let seen = configure(
        &sandbox,
        &format!("[actions.env]\nTODO_TOOL_TOKEN = {VALUE:?}\n"),
    );
    let output = invoke(&sandbox, record.id.as_str(), false);
    note::success(&output);
    assert_eq!(
        fs::read_to_string(&seen).unwrap(),
        format!("match home {}@00:00\n", record.id.as_str())
    );
    assert!(!stdout(&output).contains(VALUE));
    let state = sandbox.path().join("state/vpt");
    for file in fs::read_dir(&state).unwrap() {
        let bytes = fs::read(file.unwrap().path()).unwrap_or_default();
        assert!(!String::from_utf8_lossy(&bytes).contains(VALUE));
    }
}

#[test]
fn without_an_env_table_the_command_gets_only_the_inherited_environment() {
    let (sandbox, record) = note::configured();
    let seen = configure(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert!(
        fs::read_to_string(&seen)
            .unwrap()
            .starts_with("unset home ")
    );
}

#[test]
fn a_bad_variable_name_or_value_refuses_naming_the_key_but_not_the_value() {
    for (entry, key) in [
        (format!("'1TOKEN' = {VALUE:?}"), "actions.env.1TOKEN"),
        (
            format!("'TODO-TOKEN' = {VALUE:?}"),
            "actions.env.TODO-TOKEN",
        ),
        (
            "TODO_TOOL_TOKEN = ''".to_owned(),
            "actions.env.TODO_TOOL_TOKEN",
        ),
        (
            "TODO_TOOL_TOKEN = 3".to_owned(),
            "actions.env.TODO_TOOL_TOKEN",
        ),
    ] {
        let (sandbox, record) = note::configured();
        let seen = configure(&sandbox, &format!("[actions.env]\n{entry}\n"));
        let output = invoke(&sandbox, record.id.as_str(), false);
        assert_eq!(output.status.code(), Some(2), "{entry}");
        let error = stderr(&output);
        assert!(error.contains(key), "{error}");
        assert!(!error.contains(VALUE));
        assert!(!seen.exists());
    }
}
