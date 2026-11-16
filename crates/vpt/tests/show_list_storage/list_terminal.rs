use super::*;
use support::terminal::on_a_terminal;
use vpt_application::ports::RecordingRecord;

fn shown(sandbox: &support::Sandbox, extra: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().arg("list").args(extra),
        Duration::from_secs(3),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &support::Sandbox, extra: &[&str]) -> String {
    let output = run(sandbox.vpt().arg("list").args(extra));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn three(sandbox: &support::Sandbox) -> [RecordingRecord; 3] {
    let done = StageStates {
        transcribe: StageState::Succeeded,
        note: StageState::Succeeded,
        analysis: StageState::Disabled,
    };
    let failed = StageStates {
        transcribe: StageState::Failed,
        ..StageStates::fresh()
    };
    let rows = [
        record(sandbox, 1, done),
        record(sandbox, 2, StageStates::fresh()),
        record(sandbox, 3, failed),
    ];
    seed(sandbox, rows.to_vec());
    rows
}

#[test]
fn list_at_a_terminal_marks_each_recording_by_its_stages() {
    let sandbox = configured("list-terminal");
    let [done, pending, failed] = three(&sandbox);
    let (done, pending, failed) = (done.id, pending.id, failed.id);
    assert_eq!(
        shown(&sandbox, &[]),
        format!(
            "  ✓ {done}: title-1 (analysis disabled)\n\
             \x20 · {pending}: title-2 (transcribe pending, note pending, analysis pending)\n\
             \x20 ✗ {failed}: title-3 (transcribe failed, note pending, analysis pending)\n\
             \n\
             Needs attention\n\
             \x20 ✗ 1 issue to fix\n\
             \x20 1. {failed}: title-3 (transcribe failed, note pending, analysis pending)\n"
        )
    );
}

#[test]
fn piped_list_prints_the_plain_lines_it_always_has() {
    let sandbox = configured("list-piped");
    let [done, pending, failed] = three(&sandbox);
    assert_eq!(
        piped(&sandbox, &[]),
        format!(
            "{}  title-1  [transcribe: succeeded, note: succeeded, analysis: disabled]\n\
             {}  title-2  [transcribe: pending, note: pending, analysis: pending]\n\
             {}  title-3  [transcribe: failed, note: pending, analysis: pending]\n",
            done.id, pending.id, failed.id
        )
    );
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        assert_eq!(
            shown(&sandbox, &[format]),
            piped(&sandbox, &[format]),
            "{format}"
        );
    }
}

#[test]
fn an_empty_list_says_so_in_one_line() {
    let sandbox = configured("list-empty");
    seed(&sandbox, Vec::new());
    assert_eq!(shown(&sandbox, &[]), "  · no recordings\n");
    assert_eq!(piped(&sandbox, &[]), "no recordings\n");
}
