use crate::fixture::snapshot;
use crate::support::stderr;
use crate::{backup, deliver, ingested, machine, state, vpt};

const FIRST: i64 = 1_787_604_456;

#[test]
fn a_read_only_command_on_a_behind_ledger_warns_once_and_writes_nothing() {
    let sandbox = machine("identity-read-only");
    ingested(&sandbox, "a.m4a", FIRST);
    let older = backup(&sandbox);
    let expected = [
        vpt(&sandbox, &["list"]).stdout,
        vpt(&sandbox, &["list", "-o", "json"]).stdout,
    ];
    ingested(&sandbox, "b.m4a", FIRST + 60);
    deliver(&older, &state(&sandbox));
    let before = snapshot(&sandbox);
    for (args, expected) in [["list"].as_slice(), &["list", "-o", "json"]]
        .into_iter()
        .zip(expected)
    {
        let output = vpt(&sandbox, args);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert_eq!(output.stdout, expected);
        let warning = stderr(&output);
        assert_eq!(warning.lines().count(), 1, "{warning}");
        assert!(
            warning.starts_with("vpt: warning: ledger_behind: "),
            "{warning}"
        );
        assert!(warning.contains(&state(&sandbox).display().to_string()));
        assert!(
            warning.trim_end().ends_with("vpt ledger accept"),
            "{warning}"
        );
    }
    let shown = vpt(&sandbox, &["ledger", "show"]);
    assert!(shown.stderr.is_empty(), "{}", stderr(&shown));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn a_read_only_command_on_a_healthy_ledger_prints_no_warning() {
    let sandbox = machine("identity-read-only-healthy");
    ingested(&sandbox, "a.m4a", FIRST);
    let output = vpt(&sandbox, &["list"]);
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
}
