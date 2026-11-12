use crate::config::{ConfigError, load_text};

fn load(key: &str, value: &str) -> Result<(), ConfigError> {
    load_text(&format!("config_version = 1\n{key} = {value}")).map(drop)
}

fn out_of_range(key: &str, rule: &str) -> ConfigError {
    ConfigError::OutOfRange {
        key: key.into(),
        rule: rule.into(),
    }
}

#[test]
fn a_ratio_accepts_integer_and_float_endpoints() {
    for value in ["0", "1", "0.0", "-0.0", "0.5", "1.0"] {
        assert_eq!(load("reconcile.confidence_floor", value), Ok(()), "{value}");
    }
}

#[test]
fn a_ratio_refuses_values_outside_its_closed_range() {
    for value in [
        "-1",
        "2",
        "-0.0000000000000001",
        "1.0000000000000002",
        "inf",
        "-inf",
        "nan",
        "-nan",
    ] {
        assert_eq!(
            load("reconcile.confidence_floor", value),
            Err(out_of_range(
                "reconcile.confidence_floor",
                "a ratio in [0, 1]"
            )),
            "{value}"
        );
    }
}

#[test]
fn the_divergence_ratio_accepts_values_above_zero_through_one() {
    for value in ["0.0000000000000001", "0.5", "1", "1.0"] {
        assert_eq!(
            load("reconcile.max_divergence_ratio", value),
            Ok(()),
            "{value}"
        );
    }
}

#[test]
fn the_divergence_ratio_refuses_zero_and_outside_values() {
    for value in [
        "0",
        "0.0",
        "-0.0",
        "-1",
        "2",
        "1.0000000000000002",
        "inf",
        "-inf",
        "nan",
    ] {
        assert_eq!(
            load("reconcile.max_divergence_ratio", value),
            Err(out_of_range(
                "reconcile.max_divergence_ratio",
                "a ratio in (0, 1]"
            )),
            "{value}"
        );
    }
}

#[test]
fn positive_integers_enforce_the_32_bit_endpoints() {
    for key in [
        "source.quiet_period_secs",
        "source.deferral_page_threshold",
        "engines.timeout_secs",
        "note.slug_max_chars",
        "analysis.timeout_secs",
        "analysis.grounding_window_secs",
        "alert.aggregate_after",
    ] {
        for value in ["1", "4294967295"] {
            assert_eq!(load(key, value), Ok(()), "{key} = {value}");
        }
        for value in ["-1", "0", "4294967296", "9223372036854775807"] {
            assert_eq!(
                load(key, value),
                Err(out_of_range(key, "a positive 32-bit integer")),
                "{key} = {value}"
            );
        }
    }
}

#[test]
fn counts_gaps_and_lookbacks_enforce_the_32_bit_endpoints() {
    for key in [
        "tags.max_per_note",
        "tags.max_suggested",
        "relations.session_gap_minutes",
        "relations.max_suggested",
        "brief.lookback_days",
        "brief.max_notes",
        "brief.max_spans_per_note",
    ] {
        for value in ["0", "4294967295"] {
            assert_eq!(load(key, value), Ok(()), "{key} = {value}");
        }
        for value in ["-1", "4294967296"] {
            assert_eq!(
                load(key, value),
                Err(out_of_range(key, "a non-negative 32-bit integer")),
                "{key} = {value}"
            );
        }
    }
}

#[test]
fn audio_bytes_accepts_the_positive_64_bit_toml_range() {
    let key = "source.max_audio_bytes";
    for value in ["1", "4294967296", "9223372036854775807"] {
        assert_eq!(load(key, value), Ok(()), "{value}");
    }
    for value in ["-9223372036854775808", "-1", "0"] {
        assert_eq!(
            load(key, value),
            Err(out_of_range(key, "a positive 64-bit integer")),
            "{value}"
        );
    }
    assert!(matches!(
        load(key, "9223372036854775808"),
        Err(ConfigError::Unparseable(_))
    ));
}

#[test]
fn wrong_scalar_types_name_the_key_and_expected_type() {
    for (key, value, expected) in [
        ("source.read_titles", "\"yes\"", "boolean"),
        ("source.quiet_period_secs", "1.0", "integer"),
        ("tags.max_per_note", "false", "integer"),
        ("source.max_audio_bytes", "1.0", "integer"),
        ("reconcile.confidence_floor", "\"0.5\"", "number"),
        ("reconcile.max_divergence_ratio", "true", "number"),
        ("alert.aggregate_after", "'canary-private'", "integer"),
        ("home.path", "false", "string"),
        (
            "context.google.client_secret",
            "[\"canary-private\"]",
            "string",
        ),
    ] {
        let error = load(key, value).expect_err("configuration refused");
        assert!(!format!("{error:?}").contains("canary"));
        assert_eq!(
            error,
            ConfigError::WrongType {
                key: key.into(),
                expected: expected.into()
            }
        );
        assert_eq!(error.key(), Some(key));
    }
}

#[test]
fn enum_values_are_limited_to_the_declared_options() {
    for value in ["\"portable\"", "\"obsidian\""] {
        assert_eq!(load("note.profile", value), Ok(()));
    }
    let error = load("note.profile", "\"canary-private\"").expect_err("configuration refused");
    assert!(!format!("{error:?}").contains("canary"));
    assert_eq!(
        error,
        out_of_range("note.profile", "one of portable, obsidian")
    );
    assert_eq!(error.key(), Some("note.profile"));
}

#[test]
fn legacy_notify_keys_are_unknown() {
    for key in ["notify.mode", "notify.command", "notify.aggregate_after"] {
        assert_eq!(
            load(key, "'canary-private'"),
            Err(ConfigError::UnknownKey("notify".into()))
        );
    }
}

#[test]
fn durations_accept_zero_and_checked_unit_forms() {
    for key in ["retention.hold.audio", "brief.trigger.lead_time"] {
        for value in ["0", "\"0\"", "\"90d\"", "\"18446744073709551615s\""] {
            assert_eq!(load(key, value), Ok(()), "{key} = {value}");
        }
        for value in [
            "1",
            "-1",
            "0.0",
            "false",
            "[]",
            "\"90\"",
            "\"0d\"",
            "\"+1s\"",
            "\"18446744073709551616s\"",
            "\"307445734561825861m\"",
            "\"canary-private\"",
        ] {
            let error = load(key, value).expect_err("configuration refused");
            assert!(!format!("{error:?}").contains("canary"));
            assert_eq!(error, out_of_range(key, "0 or <n>s|m|h|d"));
        }
    }
}

#[test]
fn lists_and_argv_require_string_elements() {
    for key in ["context.calendars", "report.command", "alert.command"] {
        for value in ["[]", "[\"first\", \"second\"]"] {
            assert_eq!(load(key, value), Ok(()));
        }
        for value in ["\"first\"", "[1, 2]", "[\"canary-private\", false]", "[[]]"] {
            let error = load(key, value).expect_err("configuration refused");
            assert!(!format!("{error:?}").contains("canary"));
            assert_eq!(
                error,
                ConfigError::WrongType {
                    key: key.into(),
                    expected: "list of strings".into()
                }
            );
        }
    }
}

#[test]
fn dynamic_engine_and_language_values_use_their_declared_kinds() {
    for (key, valid, invalid, expected) in [
        ("engines.custom.local", "true", "1", "boolean"),
        (
            "engines.custom.command",
            "[\"engine\"]",
            "[1]",
            "list of strings",
        ),
        ("engines.custom.family", "\"whisper\"", "false", "string"),
        (
            "engines.by_language.fr-FR.main",
            "\"custom\"",
            "1",
            "string",
        ),
    ] {
        assert_eq!(load(key, valid), Ok(()));
        assert_eq!(
            load(key, invalid),
            Err(ConfigError::WrongType {
                key: key.into(),
                expected: expected.into()
            })
        );
    }
    assert_eq!(load("engines.custom.kind", "\"command\""), Ok(()));
    assert!(matches!(
        load("engines.custom.kind", "\"other\""),
        Err(ConfigError::OutOfRange { .. })
    ));
}

#[test]
fn refusals_without_a_key_report_none() {
    for text in ["[", "", "config_version = 2"] {
        assert_eq!(
            load_text(text)
                .map(drop)
                .expect_err("configuration refused")
                .key(),
            None
        );
    }
    assert_eq!(
        load("absent", "false")
            .expect_err("configuration refused")
            .key(),
        Some("absent")
    );
}

#[test]
fn report_and_alert_hooks_reject_invalid_argv_without_leaking_values() {
    for key in ["report.command", "alert.command"] {
        for value in ["'canary-private'", "['canary-private', false]"] {
            let error = load(key, value).expect_err("invalid argv refused");
            assert_eq!(
                error,
                ConfigError::WrongType {
                    key: key.into(),
                    expected: "list of strings".into()
                }
            );
            assert!(!format!("{error:?}").contains("canary-private"));
        }
    }
}

#[test]
fn final_exit_codes_accept_only_codes_a_command_can_exit_with() {
    let key = "actions.final_exit_codes";
    for value in ["[]", "[1]", "[2, 4, 255]"] {
        assert_eq!(load(key, value), Ok(()), "{value}");
    }
    for value in ["[0]", "[256]", "[-1]"] {
        assert_eq!(
            load(key, value),
            Err(out_of_range(key, "exit codes from 1 to 255")),
            "{value}"
        );
    }
    assert!(matches!(
        load(key, "['2']"),
        Err(ConfigError::WrongType { .. })
    ));
}
