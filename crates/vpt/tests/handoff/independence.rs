use super::*;
fn check(text: &str, path: &Path) {
    let lower = text.to_lowercase();
    for word in ["notebook_id", "5055", "8502", "surreal"] {
        assert!(!lower.contains(word), "{path:?}: {word}");
    }
    let compact = lower.replace([' ', '.', '_', '-'], "");
    assert!(!compact.contains("opennotebook"), "{path:?}");
}
fn sources(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if matches!(
            name.as_ref(),
            "tests" | "tests.rs" | "fixtures" | "fixtures.rs" | ".build"
        ) || name.ends_with("_fixture.rs")
        {
            continue;
        }
        if path.is_dir() {
            sources(&path);
        } else if path.extension().is_some_and(|s| s == "rs" || s == "swift") {
            let text = fs::read_to_string(&path).unwrap();
            let production = text.split("#[cfg(test)]").next().unwrap();
            check(production, &path);
        }
    }
}
#[test]
fn shipped_sources_and_configuration_do_not_depend_on_a_downstream_notebook_service() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for package in [
        "vpt",
        "vpt-domain",
        "vpt-application",
        "vpt-adapters",
        "vpt-protocol",
    ] {
        sources(&root.join("crates").join(package).join("src"));
    }
    sources(&root.join("helper/vpt-macos/Sources"));
    check(
        &vpt_adapters::config::template("apple", "/tmp/vpt-state"),
        Path::new("generated template"),
    );
}
