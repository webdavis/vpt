use super::actions_resend::refused;
use super::styled::{piped, same_in_every_format, shown};
use super::withdraw::{analyze, configured, keep_first_two};
use super::*;

#[test]
fn actions_resend_at_a_terminal_marks_what_was_sent_and_what_was_refused() {
    let (sandbox, id, log) = refused();
    let resend = ["actions", "resend", id.as_str()];
    let warning = "configured action command refused an action";
    assert_eq!(
        shown(&sandbox, &resend),
        format!(
            "  ⚠ actions: 4 refused\n    → send them again with vpt actions resend\n\
             \x20 ⚠ {warning}\n\nNeeds attention\n  ⚠ 2 warnings to look at\n\
             \x20 1. actions: 4 refused\n  2. {warning}\n"
        )
    );
    fs::remove_file(log.with_extension("fail")).unwrap();
    assert_eq!(shown(&sandbox, &resend), "  ✓ actions: 4 sent\n");
    assert_eq!(shown(&sandbox, &resend), "  · nothing to send\n");
    assert_eq!(piped(&sandbox, &resend), "sent 0, failed 0, refused 0\n");
    same_in_every_format(&sandbox, &resend);
}

#[test]
fn actions_resend_at_a_terminal_counts_the_withdrawals_it_finished() {
    let (sandbox, id, _, withdrawn) = configured("");
    analyze(&sandbox, &id);
    keep_first_two(&sandbox);
    fs::write(withdrawn.with_extension("fail"), "1").unwrap();
    analyze(&sandbox, &id);
    fs::remove_file(withdrawn.with_extension("fail")).unwrap();
    let resend = ["actions", "resend", id.as_str()];
    assert_eq!(shown(&sandbox, &resend), "  ✓ actions: 2 withdrawn\n");
    assert_eq!(
        piped(&sandbox, &resend),
        "sent 0, failed 0, refused 0, withdrawn 0\n"
    );
}
