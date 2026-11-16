use super::*;

const PREVIEWING: [&str; 8] = [
    "run",
    "ingest",
    "transcribe memo",
    "analyze memo",
    "tag rename old new",
    "retention run",
    "brief 2026-08-25T100000-9f8e7d6c5b4a",
    "ledger accept",
];

#[test]
fn every_previewing_command_takes_preview_and_its_short_form_alike() {
    for command in PREVIEWING {
        let long = parsed(&format!("{command} --preview")).expect(command);
        let short = parsed(&format!("{command} -n")).expect(command);
        assert_eq!(long, short, "{command}");
        assert!(
            long.verb.reads_ledger_only() || command == "ledger accept",
            "{command}"
        );
    }
}

#[test]
fn the_removed_dry_run_spelling_is_an_unknown_flag() {
    for command in PREVIEWING {
        assert_eq!(
            refused(&format!("{command} --dry-run")),
            UNKNOWN_FLAG,
            "{command}"
        );
    }
}
