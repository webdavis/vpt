use super::*;
use std::path::Path;
use std::time::Duration;
use vpt_application::engines::{CheckerRuns, EngineKind};

fn settings(extra: &str) -> Result<EngineSettings, ConfigError> {
    let table = crate::config::load_text(&format!("config_version = 1\n{extra}"))?;
    crate::config::from_table(&table, Path::new("/owned")).map(|settings| settings.engines)
}

#[test]
fn defaults_select_local_apple_and_optional_whisply() {
    let settings = settings("").unwrap();
    assert_eq!(settings.language, "en-US");
    assert_eq!(settings.pair.main, "apple");
    assert!(settings.pair.checker.is_none());
    assert_eq!(settings.checker_runs, CheckerRuns::Always);
    assert_eq!(settings.timeout, Duration::from_secs(1800));
    assert!(!settings.allow_same_family);
    assert_eq!(
        settings.definitions["whisply"].kind,
        EngineKind::Whisply {
            command: vec!["whisply".into()],
            device: "mlx".into(),
            model: "large-v3-turbo".into(),
            hf_token: None,
        }
    );
}

#[test]
fn configured_engines_and_language_overrides_reach_runtime_settings() {
    let settings = settings(
        r#"
[recording]
language = "fr-CA"
[engines]
main = "custom"
checker = "whisply"
checker_runs = "on_failure"
allow_same_family = true
timeout_secs = 27
[engines.custom]
kind = "command"
command = ["/bin/custom", "--audio", "{audio}", "--language={language}", "{out_dir}"]
family = "independent"
local = false
[engines.whisply]
command = ["/bin/whisply", "--verbose"]
device = "cpu"
model = "medium"
[engines.by_language.en-US]
main = "apple"
checker = ""
"#,
    )
    .unwrap();
    assert_eq!(settings.language, "fr-CA");
    assert_eq!(settings.pair.main, "custom");
    assert_eq!(settings.pair.checker.as_deref(), Some("whisply"));
    assert_eq!(settings.checker_runs, CheckerRuns::OnFailure);
    assert!(settings.allow_same_family);
    assert_eq!(settings.timeout, Duration::from_secs(27));
    assert_eq!(settings.by_language["en-US"].main, "apple");
    assert!(settings.by_language["en-US"].checker.is_none());
    assert!(!settings.definitions["custom"].local());
    assert!(!settings.definitions["custom"].lexical_classification());
    assert_eq!(settings.definitions["custom"].family(), "independent");
    assert_eq!(
        settings.definitions["whisply"].kind,
        EngineKind::Whisply {
            command: vec!["/bin/whisply".into(), "--verbose".into()],
            device: "cpu".into(),
            model: "medium".into(),
            hf_token: None,
        }
    );
}

#[test]
fn missing_generic_engine_contract_fields_are_key_only_errors() {
    for (tail, key) in [
        ("", "kind"),
        ("kind = 'command'", "command"),
        ("kind = 'command'\ncommand = ['tool']", "family"),
        (
            "kind = 'command'\ncommand = ['tool']\nfamily = 'private-canary'",
            "local",
        ),
    ] {
        let error = settings(&format!("[engines.custom]\n{tail}\n")).unwrap_err();
        assert_eq!(error.key(), Some(format!("engines.custom.{key}").as_str()));
        assert!(!format!("{error:?}").contains("private-canary"));
    }
}

#[test]
fn undeclared_engine_references_and_empty_commands_are_refused() {
    for (text, key) in [
        ("[engines]\nmain='unknown'", "engines.main"),
        ("[engines]\nchecker='unknown'", "engines.checker"),
        (
            "[engines.by_language.fr]\nmain='unknown'",
            "engines.by_language.fr.main",
        ),
        ("[engines.whisply]\ncommand=[]", "engines.whisply.command"),
        ("[recording]\nlanguage=''", "recording.language"),
    ] {
        assert_eq!(settings(text).unwrap_err().key(), Some(key));
    }
}
