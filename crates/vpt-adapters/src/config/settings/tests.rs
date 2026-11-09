use super::*;
use crate::config::load_text;
use vpt_domain::layout::StoreKey;
#[path = "tests/brief.rs"]
mod brief;

#[path = "tests/relations.rs"]
mod relations;
#[path = "tests/share.rs"]
mod share;
#[path = "tests/synthesis.rs"]
mod synthesis;
#[path = "tests/tags.rs"]
mod tags;

fn settings(text: &str) -> Result<Settings, ConfigError> {
    from_table(
        &load_text(&format!("config_version = 1\n{text}")).expect("valid table"),
        Path::new("/u"),
    )
}

#[test]
fn defaults_expand_into_typed_settings() {
    let value = settings("").expect("settings");
    assert_eq!(value.config_version, 1);
    assert_eq!(value.home, Path::new("/u/.vpt"));
    assert_eq!(value.state_dir, Path::new("/u/.local/state/vpt"));
    assert_eq!(value.symlink_target, None);
    assert_eq!(value.helper_path, Path::new("vpt-macos"));
    for (key, leaf) in [
        (StoreKey::Audio, "audio"),
        (StoreKey::Transcripts, "transcripts"),
        (StoreKey::Analysis, "analysis"),
        (StoreKey::Briefs, "briefs"),
        (StoreKey::EngineOutputs, "engine-outputs"),
        (StoreKey::Drafts, "drafts"),
        (StoreKey::Released, "released"),
    ] {
        assert_eq!(value.stores.get(key), Path::new("/u/.vpt").join(leaf));
        assert_eq!(value.retention.hold(key).seconds(), 0);
    }
    assert_eq!(
        value.source.recordings_dir,
        Path::new("/u/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings")
    );
    assert!(value.source.read_titles);
    assert_eq!(value.source.quiet_period_secs, 30);
    assert_eq!(value.source.deferral_page_threshold, 4);
    assert_eq!(value.source.max_audio_bytes, 2_147_483_648);
    assert!(value.report_command.is_empty());
    assert!(value.alert_command.is_empty());
    assert_eq!(value.alert_aggregate_after, 3);
    assert!(!value.retention.enabled);
    assert!(!value.retention.include_audio);
    assert_eq!(
        value.reconcile.options,
        vpt_domain::uncertainty::Options::default()
    );
    assert_eq!(
        value.reconcile.known_terms_path,
        Path::new("/u/.config/vpt/known-terms.txt")
    );
    assert!(!value.readability_enabled);
}

#[test]
fn configured_reconciliation_and_readability_reach_runtime_settings() {
    let value = settings("[reconcile]\nconfidence_floor = 0.2\nmax_divergence_ratio = 1\nknown_terms_path = '~/confirmed.txt'\n[readability]\nenabled = true").unwrap();
    assert_eq!(value.reconcile.options.confidence_floor, 0.2);
    assert_eq!(value.reconcile.options.max_divergence_ratio, 1.0);
    assert_eq!(
        value.reconcile.known_terms_path,
        Path::new("/u/confirmed.txt")
    );
    assert!(value.readability_enabled);
}

#[test]
fn configured_paths_and_numeric_limits_survive_conversion() {
    let value = settings(
        r#"
[home]
path = "~"
state_dir = "/state"
[stores]
audio = "~/audio"
engine_outputs = "<home>/raw"
briefs = "relative"
[source]
recordings_dir = "~/source/Recordings"
read_titles = false
quiet_period_secs = 4294967295
deferral_page_threshold = 4294967295
max_audio_bytes = 9223372036854775807
[helper]
path = "/bin/helper"
[alert]
aggregate_after = 4294967295
"#,
    )
    .expect("settings");
    assert_eq!(value.home, Path::new("/u"));
    assert_eq!(value.state_dir, Path::new("/state"));
    assert_eq!(value.stores.audio, Path::new("/u/audio"));
    assert_eq!(value.stores.engine_outputs, Path::new("/u/raw"));
    assert_eq!(value.stores.briefs, Path::new("relative"));
    assert_eq!(
        value.source.recordings_dir,
        Path::new("/u/source/Recordings")
    );
    assert!(!value.source.read_titles);
    assert_eq!(value.source.quiet_period_secs, 4_294_967_295);
    assert_eq!(value.source.deferral_page_threshold, u32::MAX);
    assert_eq!(value.source.max_audio_bytes, 9_223_372_036_854_775_807);
    assert_eq!(value.helper_path, Path::new("/bin/helper"));
    assert_eq!(value.alert_aggregate_after, u32::MAX);
}

#[test]
fn report_and_alert_commands_preserve_argv_and_accept_empty() {
    let value = settings(
        "[report]\ncommand = ['reporter', 'two words']\n[alert]\ncommand = ['alerter', '--urgent']",
    )
    .expect("settings");
    assert_eq!(
        value.report_command,
        vec!["reporter".to_owned(), "two words".to_owned()]
    );
    assert_eq!(
        value.alert_command,
        vec!["alerter".to_owned(), "--urgent".to_owned()]
    );
    let empty = settings("[report]\ncommand = []\n[alert]\ncommand = []").expect("settings");
    assert!(empty.report_command.is_empty());
    assert!(empty.alert_command.is_empty());
}

#[test]
fn each_hook_can_be_left_empty_independently() {
    let only_report = settings("[report]\ncommand = ['reporter']").expect("settings");
    assert_eq!(only_report.report_command, vec!["reporter"]);
    assert!(only_report.alert_command.is_empty());
    let only_alert = settings("[alert]\ncommand = ['alerter']").expect("settings");
    assert!(only_alert.report_command.is_empty());
    assert_eq!(only_alert.alert_command, vec!["alerter"]);
}

#[test]
fn managed_link_requires_the_default_home() {
    assert_eq!(
        settings("[home]\npath = '/elsewhere'\nsymlink_target = '/target'")
            .unwrap_err()
            .key(),
        Some("home.symlink_target")
    );
    assert_eq!(
        settings("[home]\nsymlink_target = '~/target'")
            .expect("settings")
            .symlink_target,
        Some(Path::new("/u/target").into())
    );
}

#[test]
fn retention_holds_keep_each_store_and_never_defaults() {
    let value = settings("[retention]\nenabled = true\ninclude_audio = true\n[retention.hold]\naudio = '2d'\ntranscripts = '3h'\nanalysis = '4m'\nbriefs = '5s'\nengine_outputs = '18446744073709551615s'\ndrafts = '0'").expect("settings");
    assert!(value.retention.enabled);
    assert!(value.retention.include_audio);
    for (key, seconds) in [
        (StoreKey::Audio, 172_800),
        (StoreKey::Transcripts, 10_800),
        (StoreKey::Analysis, 240),
        (StoreKey::Briefs, 5),
        (StoreKey::EngineOutputs, u64::MAX),
        (StoreKey::Drafts, 0),
        (StoreKey::Released, 0),
    ] {
        assert_eq!(value.retention.hold(key).seconds(), seconds);
    }
    let mut retention = value.retention;
    retention.holds.clear();
    assert_eq!(retention.hold(StoreKey::Audio).seconds(), 0);
}
