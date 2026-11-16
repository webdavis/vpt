use super::*;

#[test]
fn an_empty_summary_says_so_in_one_line() {
    assert_eq!(
        Summary::default().finish("nothing to do"),
        "  · nothing to do\n"
    );
}

#[test]
fn rows_that_only_succeeded_need_no_closing_section() {
    let mut summary = Summary::default();
    summary.row(Verdict::Ok, "ingested", "2 recordings");
    summary.row(Verdict::NotRun, "a.m4a", "would ingest");
    assert_eq!(
        summary.finish("nothing to do"),
        "  ✓ ingested: 2 recordings\n  · a.m4a: would ingest\n"
    );
}

#[test]
fn a_warning_or_failure_closes_with_what_needs_attention() {
    let mut summary = Summary::default();
    summary
        .row(Verdict::Warning, "actions", "1 refused")
        .advice("send them again");
    summary.mark(Verdict::Issue, "the engine stopped");
    assert_eq!(
        summary.finish("nothing to do"),
        "  ⚠ actions: 1 refused\n    → send them again\n  ✗ the engine stopped\n\n\
         Needs attention\n  ✗ 1 issue to fix, 1 warning to look at\n\
         \x20 1. the engine stopped\n  2. actions: 1 refused\n"
    );
}

#[test]
fn counts_name_one_thing_in_the_singular() {
    assert_eq!(counted(1, "file", "files"), "1 file");
    assert_eq!(counted(0, "file", "files"), "0 files");
}
