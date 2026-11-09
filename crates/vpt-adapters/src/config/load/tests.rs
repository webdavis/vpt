use super::*;
use crate::config::{keys, template};

#[test]
fn the_rendered_template_loads_and_every_key_of_the_table_is_present() {
    let table = load_text(&template("apple", "~/.local/state/vpt")).expect("template loads");
    for key in keys() {
        assert!(
            get(&table, key.path).is_some(),
            "{} missing after load",
            key.path
        );
    }
    assert_eq!(
        get(&table, "engines.main").and_then(|v| v.as_str()),
        Some("apple")
    );
}

#[test]
fn an_operator_value_wins_over_the_default() {
    let table = load_text("config_version = 1\n[source]\nquiet_period_secs = 90\n").expect("loads");
    assert_eq!(
        get(&table, "source.quiet_period_secs").and_then(|v| v.as_integer()),
        Some(90)
    );
    assert_eq!(
        get(&table, "source.deferral_page_threshold").and_then(|v| v.as_integer()),
        Some(4)
    );
}

#[test]
fn report_and_alert_hooks_default_to_empty_argv() {
    let table = load_text("config_version = 1\n").expect("defaults load");
    for key in ["report.command", "alert.command"] {
        assert_eq!(
            get(&table, key).and_then(Value::as_array),
            Some(&Vec::new())
        );
    }
    assert_eq!(
        get(&table, "alert.aggregate_after").and_then(Value::as_integer),
        Some(3)
    );
}

#[test]
fn report_and_alert_tables_preserve_operator_argv() {
    let table = load_text(
        "config_version = 1\n[report]\ncommand = ['reporter', 'two words']\n\
         [alert]\ncommand = ['alerter', '--urgent']\naggregate_after = 7\n",
    )
    .expect("hooks load");
    for (key, first, second) in [
        ("report.command", "reporter", "two words"),
        ("alert.command", "alerter", "--urgent"),
    ] {
        let argv = get(&table, key)
            .and_then(Value::as_array)
            .expect("hook argv");
        assert_eq!(argv[0].as_str(), Some(first));
        assert_eq!(argv[1].as_str(), Some(second));
    }
    assert_eq!(
        get(&table, "alert.aggregate_after").and_then(Value::as_integer),
        Some(7)
    );
}

#[test]
fn legacy_notify_table_is_refused() {
    assert_eq!(
        load_text("config_version = 1\n[notify]\nmode = 'off'\n")
            .map(drop)
            .expect_err("legacy table refused"),
        ConfigError::UnknownKey("notify".into())
    );
}

#[test]
fn an_unknown_key_is_refused_by_its_full_path() {
    let error = load_text("config_version = 1\n[source]\nquiet_period = 90\n")
        .map(drop)
        .expect_err("configuration refused");
    assert_eq!(error, ConfigError::UnknownKey("source.quiet_period".into()));
}

#[test]
fn an_unknown_table_is_refused_even_when_empty() {
    assert_eq!(
        load_text("config_version = 1\n[unknown]\n")
            .map(drop)
            .expect_err("configuration refused"),
        ConfigError::UnknownKey("unknown".into())
    );
    assert_eq!(
        load_text("config_version = 1\n[source.extra]\n")
            .map(drop)
            .expect_err("configuration refused"),
        ConfigError::UnknownKey("source.extra".into())
    );
}

#[test]
fn a_table_where_a_scalar_belongs_is_a_wrong_type() {
    let error = load_text("config_version = 1\n[source]\nquiet_period_secs = {}\n")
        .map(drop)
        .expect_err("configuration refused");
    assert_eq!(
        error,
        ConfigError::WrongType {
            key: "source.quiet_period_secs".into(),
            expected: "integer".into()
        }
    );
}

#[test]
fn a_missing_config_version_is_refused() {
    assert_eq!(
        load_text("[source]\nread_titles = false\n")
            .map(drop)
            .expect_err("configuration refused"),
        ConfigError::MissingVersion
    );
}

#[test]
fn a_future_config_version_is_refused_with_its_value() {
    assert_eq!(
        load_text("config_version = 2\n")
            .map(drop)
            .expect_err("configuration refused"),
        ConfigError::UnsupportedVersion(2)
    );
}

#[test]
fn an_engine_table_and_a_language_pair_are_dynamic_keys() {
    let text = "config_version = 1\n[engines.cloudy]\nkind = \"command\"\ncommand = [\"x\"]\n\
                family = \"whisper\"\nlocal = false\n[engines.by_language.de-DE]\nmain = \"cloudy\"\n\
                [retention.hold]\naudio = \"90d\"\n";
    let table = load_text(text).expect("dynamic keys load");
    assert_eq!(
        get(&table, "engines.cloudy.family").and_then(|v| v.as_str()),
        Some("whisper")
    );
    assert_eq!(
        get(&table, "engines.by_language.de-DE.main").and_then(|v| v.as_str()),
        Some("cloudy")
    );
}

#[test]
fn a_dynamic_table_still_refuses_a_leaf_it_does_not_know() {
    let error = load_text("config_version = 1\n[engines.cloudy]\nkind = \"command\"\nspeed = 3\n")
        .map(drop)
        .expect_err("configuration refused");
    assert_eq!(
        error,
        ConfigError::UnknownKey("engines.cloudy.speed".into())
    );
}

#[test]
fn a_hold_for_an_unknown_store_is_refused() {
    let error = load_text("config_version = 1\n[retention.hold]\nvideos = \"1d\"\n")
        .map(drop)
        .expect_err("configuration refused");
    assert_eq!(
        error,
        ConfigError::UnknownKey("retention.hold.videos".into())
    );
}

#[test]
fn unparseable_toml_is_refused_without_quoting_the_text() {
    let text = "config_version = 1\n[context.google]\nclient_secret = \"canary-7Q9x-secret\n";
    let error = load_text(text)
        .map(drop)
        .expect_err("configuration refused");
    assert!(
        !format!("{error:?}").contains("canary"),
        "configuration contents leaked"
    );
    assert_eq!(
        error,
        ConfigError::Unparseable("invalid TOML syntax in configuration".into())
    );
    assert!(!format!("{error:?}").contains(text));
}

#[test]
fn a_state_path_with_a_quote_and_a_backslash_round_trips_through_the_template() {
    let table = load_text(&template("apple", "/tmp/say \"hi\"\\now")).expect("loads");
    assert_eq!(
        get(&table, "home.state_dir").and_then(|v| v.as_str()),
        Some("/tmp/say \"hi\"\\now")
    );
}

#[test]
fn a_noninteger_version_never_leaks_its_secret_value() {
    let error = load_text("config_version = \"canary-version-private\"")
        .map(drop)
        .expect_err("configuration refused");
    assert!(
        !format!("{error:?}").contains("canary"),
        "configuration contents leaked"
    );
    assert_eq!(
        error,
        ConfigError::WrongType {
            key: "config_version".into(),
            expected: "integer".into()
        }
    );
}

#[test]
fn scalar_replacement_of_a_declared_table_is_refused() {
    for key in [
        "source",
        "engines",
        "engines.by_language",
        "engines.custom",
        "engines.by_language.de-DE",
    ] {
        let error = load_text(&format!(
            "config_version = 1\n{key} = \"canary-table-private\""
        ))
        .map(drop)
        .expect_err("configuration refused");
        assert!(!format!("{error:?}").contains("canary"));
        assert_eq!(
            error,
            ConfigError::WrongType {
                key: key.into(),
                expected: "table".into()
            }
        );
    }
}

#[test]
fn dotted_literal_segments_cannot_impersonate_schema_paths() {
    for text in [
        "config_version = 1\n\"source.read_titles\" = false",
        "config_version = 1\n[\"source.read_titles\"]",
        "config_version = 1\n[engines.\"custom.command\"]\nkind = \"command\"",
    ] {
        assert!(matches!(load_text(text), Err(ConfigError::UnknownKey(_))));
    }
}

#[test]
fn language_table_cannot_be_used_as_an_engine() {
    let error = load_text("config_version = 1\n[engines.by_language]\nkind = \"command\"")
        .map(drop)
        .expect_err("configuration refused");
    assert_eq!(
        error,
        ConfigError::WrongType {
            key: "engines.by_language.kind".into(),
            expected: "table".into()
        }
    );
}

#[test]
fn file_loading_reads_only_the_selected_fixture_and_classifies_failures() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("config.toml");
    assert_eq!(
        load_file(&path),
        Err(ConfigError::Missing(path.display().to_string()))
    );
    std::fs::write(
        &path,
        "config_version = 1\n[source]\nquiet_period_secs = 91",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let table = load_file(&path).unwrap();
    assert_eq!(
        get(&table, "source.quiet_period_secs").and_then(Value::as_integer),
        Some(91)
    );
    std::fs::write(&path, b"canary-private\xff").unwrap();
    let error = load_file(&path)
        .map(drop)
        .expect_err("configuration refused");
    assert!(
        matches!(&error, ConfigError::Unreadable { path: p, detail } if p == &path.display().to_string() && !detail.is_empty())
    );
    assert!(!format!("{error:?}").contains("canary"));
    std::fs::write(
        &path,
        "config_version = 1\n[context.google]\nclient_secret = \"canary-file-private",
    )
    .unwrap();
    let error = load_file(&path)
        .map(drop)
        .expect_err("configuration refused");
    assert!(
        !format!("{error:?}").contains("canary"),
        "configuration contents leaked"
    );
    assert_eq!(
        error,
        ConfigError::Unparseable("invalid TOML syntax in configuration".into())
    );
}

#[test]
fn missing_paths_and_scalar_intermediates_return_none() {
    let table: Table = "config_version = 1\n[source]\nread_titles = false"
        .parse()
        .unwrap();
    for key in ["", "absent", "source.absent", "source.read_titles.absent"] {
        assert!(get(&table, key).is_none());
    }
}

#[test]
fn table_values_report_the_declared_leaf_kind() {
    for (key, expected) in [
        ("source.read_titles", "boolean"),
        ("source.quiet_period_secs", "integer"),
        ("source.max_audio_bytes", "integer"),
        ("tags.max_per_note", "integer"),
        ("reconcile.confidence_floor", "number"),
        ("reconcile.max_divergence_ratio", "number"),
        ("retention.hold.audio", "duration"),
        ("note.profile", "string"),
        ("home.path", "string"),
        ("context.google.client_secret", "string"),
        ("context.calendars", "list of strings"),
        ("synthesis.command", "list of strings"),
    ] {
        let error = load_text(&format!("config_version = 1\n{key} = {{}}"))
            .map(drop)
            .expect_err("configuration refused");
        assert_eq!(
            error,
            ConfigError::WrongType {
                key: key.into(),
                expected: expected.into()
            }
        );
    }
}
