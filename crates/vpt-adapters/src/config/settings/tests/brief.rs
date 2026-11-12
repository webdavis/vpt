use super::*;
use std::time::Duration;
use vpt_application::BriefSettings;
use vpt_domain::brief::Options;

#[test]
fn brief_defaults_and_configured_limits_reach_runtime_settings() {
    assert_eq!(settings("").unwrap().brief, BriefSettings::default());
    let configured = settings("[brief]\nmax_notes=3\nlookback_days=7\nmax_spans_per_note=2\n[brief.trigger]\nenabled=true\nlead_time='90m'").unwrap().brief;
    assert_eq!(
        configured.options,
        Options {
            max_notes: 3,
            lookback_days: 7,
            max_spans_per_note: 2,
        }
    );
    assert!(configured.trigger_enabled);
    assert_eq!(configured.lead_time, Duration::from_secs(5400));
}

#[test]
fn zero_brief_caps_and_an_empty_trigger_window_remain_explicit() {
    let configured = settings(
        "[brief]\nmax_notes=0\nlookback_days=0\nmax_spans_per_note=0\n[brief.trigger]\nlead_time=0",
    )
    .unwrap()
    .brief;
    assert_eq!(
        configured.options,
        Options {
            max_notes: 0,
            lookback_days: 0,
            max_spans_per_note: 0,
        }
    );
    assert_eq!(configured.lead_time, Duration::ZERO);
}

#[test]
fn brief_counts_accept_the_full_configured_32_bit_range() {
    for value in [65_536, 65_537, u32::MAX] {
        let configured = settings(&format!(
            "[brief]\nmax_notes={value}\nmax_spans_per_note={value}"
        ))
        .unwrap()
        .brief;
        assert_eq!(configured.options.max_notes, value as usize);
        assert_eq!(configured.options.max_spans_per_note, value as usize);
    }
}

#[test]
fn context_provider_scopes_and_credentials_reach_settings_without_private_debug_text() {
    let settings=settings("[context]\ntype='google'\ncalendars=['Private calendar']\nprojects=['Private project']\n[context.google]\nclient_id='Private client'\nclient_secret='Private secret'\nrefresh_token='Private token'").unwrap();
    assert_eq!(
        settings.context.source,
        vpt_application::ContextSource::Google
    );
    assert_eq!(settings.context.calendars, ["Private calendar"]);
    assert_eq!(settings.context.google.refresh_token, "Private token");
    assert!(!format!("{settings:?}").contains("Private"));
    assert!(!format!("{:?}", settings.context.google).contains("Private"));
}

#[test]
fn dam_max_age_defaults_to_none_and_reaches_settings() {
    assert!(settings("").unwrap().context.dam_max_age.is_empty());
    let settings = settings("[context.dam]\nmax_age=['gcal=15m']").unwrap();
    assert_eq!(settings.context.dam_max_age, ["gcal=15m"]);
}

#[test]
fn context_scopes_and_provider_names_are_bounded_and_validated() {
    for config in [
        "[context]\ntype='Private invalid'".to_owned(),
        "[context]\ncalendars=['']".to_owned(),
        format!("[context]\nprojects=[{}]", vec!["'private'"; 257].join(",")),
    ] {
        let error = load_text(&format!("config_version = 1\n{config}"))
            .and_then(|table| from_table(&table, Path::new("/u")))
            .unwrap_err();
        assert!(matches!(error, ConfigError::OutOfRange { .. }));
        assert!(!format!("{error:?}").contains("Private"));
    }
}
