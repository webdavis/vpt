use super::*;
use serde_json::json;

#[test]
fn bulk_redo_sums_the_action_counts_of_every_recording_it_ran() {
    let (sandbox, rows) = note::configured_with(&[7, 8]);
    let log = actions::configure(&sandbox);
    let result = note::success(&batch::bulk(&sandbox, &["--all"]));
    assert_eq!(result["recordings"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["actions"],
        json!({"sent": 8, "failed": 0, "refused": 0})
    );
    assert_eq!(actions::sent(&log).len(), 8);
    let input = format!("{}\n{}\n", rows[0].id, rows[1].id);
    let output = batch_input::piped(&sandbox, &input, &[]);
    assert_eq!(
        note::success(&output)["actions"],
        json!({"sent": 0, "failed": 0, "refused": 0})
    );
}

#[test]
fn bulk_redo_reports_no_action_counts_without_a_command_or_on_a_dry_run() {
    let (sandbox, _rows) = note::configured_with(&[7, 8]);
    agent(&sandbox, "");
    let result = note::success(&batch::bulk(&sandbox, &["--all"]));
    assert!(result.get("actions").is_none(), "{result}");
    actions::configure(&sandbox);
    let result = note::success(&batch::bulk(&sandbox, &["--all", "--dry-run"]));
    assert!(result.get("actions").is_none(), "{result}");
}
