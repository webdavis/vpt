use super::*;
use std::time::Duration;
use support::terminal::on_a_terminal;

const BUDGET: Duration = Duration::from_secs(10);

pub(super) fn shown(sandbox: &Sandbox, args: &[&str]) -> String {
    let output = on_a_terminal(sandbox.vpt().args(args), BUDGET);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

pub(super) fn piped(sandbox: &Sandbox, args: &[&str]) -> String {
    let output = support::run_with_budget(sandbox.vpt().args(args), BUDGET);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

pub(super) fn document(sandbox: &Sandbox, args: &[&str]) -> serde_json::Value {
    serde_json::from_str(&piped(sandbox, &[args, &["-o", "json"]].concat())).unwrap()
}

pub(super) fn same_in_every_format(sandbox: &Sandbox, args: &[&str]) {
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        let args = [args, &[format]].concat();
        assert_eq!(shown(sandbox, &args), piped(sandbox, &args), "{format}");
    }
}
