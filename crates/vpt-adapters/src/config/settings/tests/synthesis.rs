use super::*;
use std::time::Duration;

#[test]
fn synthesis_defaults_and_configured_values_reach_runtime_settings() {
    let defaults = settings("").unwrap().synthesis;
    assert!(defaults.command.is_empty());
    assert_eq!(defaults.timeout, Duration::from_secs(600));
    assert_eq!(defaults.grounding_window_secs, 15);
    let configured = settings("[synthesis]\ncommand=['Private executable', '{transcript}', '--input={id}', '{language}']\ntimeout_secs=17\ngrounding_window_secs=4").unwrap().synthesis;
    assert_eq!(
        configured.command,
        [
            "Private executable",
            "{transcript}",
            "--input={id}",
            "{language}"
        ]
    );
    assert_eq!(configured.timeout, Duration::from_secs(17));
    assert_eq!(configured.grounding_window_secs, 4);
    assert!(!format!("{configured:?}").contains("Private"));
}

#[test]
fn invalid_synthesis_command_arguments_are_refused_without_echoing_them() {
    for command in [
        "['']",
        "['   ', 'Private argument']",
        r#"["Private\u0000program"]"#,
    ] {
        let table = load_text(&format!("config_version=1\n[synthesis]\ncommand={command}"));
        let error = table
            .and_then(|table| from_table(&table, Path::new("/u")))
            .unwrap_err();
        assert!(!format!("{error:?}").contains("Private"));
    }
}
