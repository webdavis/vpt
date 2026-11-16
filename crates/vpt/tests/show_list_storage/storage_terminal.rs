use super::*;
use support::terminal::on_a_terminal;

fn shown(sandbox: &support::Sandbox, extra: &[&str]) -> String {
    let output = on_a_terminal(
        sandbox.vpt().arg("storage").args(extra),
        Duration::from_secs(3),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn piped(sandbox: &support::Sandbox, extra: &[&str]) -> String {
    let output = run(sandbox.vpt().arg("storage").args(extra));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

fn with_audio(name: &str) -> support::Sandbox {
    let sandbox = configured(name);
    let audio = sandbox.path().join("home/.vpt/audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join("one.m4a"), b"abc").unwrap();
    std::fs::write(audio.join("two.m4a"), b"de").unwrap();
    sandbox
}

#[test]
fn storage_at_a_terminal_marks_each_store_with_its_files_and_path() {
    let sandbox = with_audio("storage-terminal");
    let stores = json(&run(sandbox.vpt().args(["storage", "-o", "json"])))["stores"].clone();
    let expected: String = stores
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, store)| {
            let files = match store["files"].as_u64().unwrap() {
                1 => "1 file".to_owned(),
                count => format!("{count} files"),
            };
            let path = store["path"].as_str().unwrap();
            format!("  ✓ {name}: {files}, {} bytes, {path}\n", store["bytes"])
        })
        .collect();
    assert!(expected.starts_with("  ✓ audio: 2 files, 5 bytes, "));
    assert_eq!(shown(&sandbox, &[]), expected);
}

#[test]
fn piped_storage_prints_the_plain_lines_it_always_has() {
    let sandbox = with_audio("storage-piped");
    let audio = sandbox.path().join("home/.vpt/audio");
    let first = piped(&sandbox, &[]).lines().next().unwrap().to_owned();
    assert!(first.starts_with("audio                2 files            5 bytes  "));
    assert!(first.ends_with(audio.file_name().unwrap().to_str().unwrap()));
    for format in ["--output=json", "--output=toon", "--output=yaml"] {
        assert_eq!(
            shown(&sandbox, &[format]),
            piped(&sandbox, &[format]),
            "{format}"
        );
    }
}
