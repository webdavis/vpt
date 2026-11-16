use super::*;
use support::terminal::on_a_terminal;

fn shown(sandbox: &support::Sandbox, extra: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().arg("show").args(extra),
        Duration::from_secs(3),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &support::Sandbox, extra: &[&str]) -> String {
    let output = run(sandbox.vpt().arg("show").args(extra));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn failed(name: &str) -> (support::Sandbox, String) {
    let sandbox = configured(name);
    let stages = StageStates {
        transcribe: StageState::Failed,
        ..StageStates::fresh()
    };
    let row = record(&sandbox, 1, stages);
    seed(&sandbox, vec![row.clone()]);
    (sandbox, row.id.as_str().to_owned())
}

#[test]
fn show_at_a_terminal_lists_the_recording_facts_as_rows() {
    let (sandbox, id) = failed("show-terminal");
    let facts = json(&run(sandbox.vpt().args(["show", &id, "-o", "json"])));
    let captured = facts["captured_at"].as_str().unwrap();
    let audio = facts["audio_path"].as_str().unwrap();
    assert_eq!(
        shown(&sandbox, &[&id]),
        format!(
            "  ✓ {id}: title-1\n\
             \x20 ✓ captured: {captured}, 19 seconds\n\
             \x20 ✗ transcribe: failed\n\
             \x20 · note: pending\n\
             \x20 · analysis: pending\n\
             \x20 ✓ audio: {audio}\n\
             \n\
             Needs attention\n\
             \x20 ✗ 1 issue to fix\n\
             \x20 1. transcribe: failed\n"
        )
    );
}

#[test]
fn piped_show_prints_the_record_it_always_has() {
    let (sandbox, id) = failed("show-piped");
    let plain = piped(&sandbox, &[&id]);
    assert!(plain.starts_with(&format!("{{\n  \"id\": \"{id}\",\n")));
    assert!(plain.contains("\n  \"title\": \"title-1\",\n"));
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        assert_eq!(
            shown(&sandbox, &[&id, format]),
            piped(&sandbox, &[&id, format]),
            "{format}"
        );
    }
}
