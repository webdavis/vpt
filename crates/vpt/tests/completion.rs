mod support;

use support::{Sandbox, run, stderr, stdout};

#[test]
fn each_shell_gets_a_completion_script_for_vpt() {
    let sandbox = Sandbox::new("completion-shells");
    for (shell, marker) in [
        ("bash", "complete -F _vpt"),
        ("zsh", "#compdef vpt"),
        ("fish", "complete -c vpt"),
    ] {
        let output = run(sandbox.vpt().args(["completion", shell]));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{shell}: {}",
            stderr(&output)
        );
        let script = stdout(&output);
        assert!(script.contains(marker), "{shell}");
        assert!(script.contains("transcribe"), "{shell}");
    }
}

#[test]
fn an_unknown_shell_is_a_usage_error() {
    let sandbox = Sandbox::new("completion-unknown");
    let output = run(sandbox.vpt().args(["completion", "tcsh"]));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
}
