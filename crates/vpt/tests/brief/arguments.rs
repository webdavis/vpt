use super::*;
#[test]
fn invalid_brief_arguments_and_disabled_trigger_are_observational_and_private() {
    let sandbox = fixture::configured("brief-arguments");
    let before = fixture::snapshot(&sandbox);
    for args in [
        vec!["brief"],
        vec!["brief", "--title", "Private secret"],
        vec![
            "brief",
            "--title",
            "Private secret",
            "--at",
            "Private invalid",
        ],
        vec![
            "brief",
            "--title",
            "Private secret",
            "--at",
            "2026-10-01T00:00:00Z",
            "--duration-minutes",
            "18446744073709551615",
        ],
        vec!["brief", "--upcoming"],
        vec!["brief", "--upcoming", "--dry-run"],
        vec!["brief", "--at", "2026-10-01T00:00:00Z"],
        vec![
            "brief",
            "2026-10-01T000000-aaaaaaaaaaaa",
            "--tag",
            "Private secret",
        ],
    ] {
        let mut args = args;
        args.push("--json");
        let output = invoke(&sandbox, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(!stderr(&output).contains("Private"));
        assert!(output.stdout.is_empty());
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}
#[test]
fn option_like_title_and_tags_remain_literal_private_values() {
    let sandbox = fixture::configured("brief-literal");
    let before = fixture::snapshot(&sandbox);
    let result = json(&invoke(
        &sandbox,
        &[
            "brief",
            "--title",
            "--json",
            "--at",
            "2026-10-01T00:00:00Z",
            "--tag",
            "--upcoming",
            "--participant",
            "--dry-run",
            "--dry-run",
            "--json",
        ],
    ));
    assert_eq!(result["occasion"]["title"], "--json");
    assert_eq!(result["occasion"]["participants"][0]["name"], "--dry-run");
    assert_eq!(fixture::snapshot(&sandbox), before);
}
#[test]
fn configured_context_is_never_silently_treated_as_disabled() {
    let sandbox = fixture::configured("brief-context-scopes");
    fixture::write_config(&sandbox, "[context]\ntype='dam'\n");
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &manual(&["--dry-run"]));
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(stderr(&output).contains("context.calendars"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}
#[test]
fn unknown_existing_occasion_does_not_create_state() {
    let sandbox = fixture::configured("brief-unknown");
    let before = fixture::snapshot(&sandbox);
    let output = invoke(
        &sandbox,
        &["brief", "2026-10-01T000000-aaaaaaaaaaaa", "--json"],
    );
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(fixture::snapshot(&sandbox), before);
}
