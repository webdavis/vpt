use crate::support::{Sandbox, run, stderr, stdout};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use vpt_application::engine_trial::SENTENCE;

pub(super) struct Trial {
    pub(super) sandbox: Sandbox,
    pub(super) temp: tempfile::TempDir,
}

impl Trial {
    pub(super) fn new(name: &str) -> Self {
        let sandbox = crate::observations_fixture::configured(name);
        sandbox.install_fake_helper();
        let clip = sandbox.path().join("clip-fixture.m4a");
        std::fs::write(&clip, vpt_domain::fixtures::m4a(0, 9, b"made-up speech")).unwrap();
        script(
            &sandbox,
            "say",
            &format!("[ \"$1\" = -o ] || exit 64\n/bin/cp {clip:?} \"$2\""),
        );
        let temp = tempfile::Builder::new()
            .prefix("vpt-self-test-tmp-")
            .tempdir_in(std::env::temp_dir().canonicalize().unwrap())
            .unwrap();
        let trial = Self { sandbox, temp };
        trial.document("apple", "apple", true, SENTENCE);
        trial
    }

    pub(super) fn path(&self) -> &Path {
        self.sandbox.path()
    }

    pub(super) fn configure(&self, extra: &str) {
        crate::observations_fixture::write_config(&self.sandbox, extra);
    }

    pub(super) fn engine(&self, name: &str, local: bool, body: &str) -> String {
        let program = script(&self.sandbox, name, body);
        format!(
            "[engines.{name}]\nkind = 'command'\nfamily = '{name}-family'\nlocal = {local}\ncommand = [{program:?}]\n"
        )
    }

    pub(super) fn document(&self, name: &str, family: &str, local: bool, text: &str) -> PathBuf {
        let path = self.path().join(format!("{name}-document.json"));
        let document = json!({
            "schema": "vpt.engine/1",
            "engine": {"name": name, "tool": "fixture", "version": "1", "model": "test",
                "family": family},
            "language": "en-US", "local": local,
            "segments": [{"start": 0.0, "end": 8.0, "text": text,
                "words": [{"text": text, "start": 0.0, "end": 8.0}]}]
        });
        std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        path
    }

    pub(super) fn hearing(&self, name: &str, local: bool, text: &str) -> String {
        let document = self.document(name, &format!("{name}-family"), local, text);
        self.engine(name, local, &format!("/bin/cat {document:?}"))
    }

    pub(super) fn output(&self, args: &[&str]) -> Output {
        run(self
            .sandbox
            .vpt()
            .current_dir(self.path())
            .env("TMPDIR", self.temp.path())
            .env("VPT_FAKE_DOCUMENT", self.path().join("apple-document.json"))
            .args(args))
    }

    pub(super) fn json(&self, args: &[&str], code: i32) -> Value {
        let output = self.output(&[args, &["--json"]].concat());
        assert_eq!(output.status.code(), Some(code), "{}", stderr(&output));
        assert!(output.stderr.is_empty(), "{}", stderr(&output));
        let document: Value = serde_json::from_str(&stdout(&output)).unwrap();
        assert_eq!(document["schema"], "vpt.result/1");
        assert_eq!(document["command"], "test");
        assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
        document
    }

    pub(super) fn temp_is_empty(&self) -> bool {
        std::fs::read_dir(self.temp.path())
            .unwrap()
            .next()
            .is_none()
    }
}

pub(super) fn script(sandbox: &Sandbox, name: &str, body: &str) -> PathBuf {
    let program = sandbox.path().join("bin").join(name);
    std::fs::write(&program, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    program
}

pub(super) fn rows(document: &Value) -> Vec<&Value> {
    let checks = document["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1, "{document}");
    assert_eq!(checks[0]["name"], "engines");
    checks[0]["rows"].as_array().unwrap().iter().collect()
}

pub(super) fn row<'a>(document: &'a Value, name: &str) -> &'a Value {
    rows(document)
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row {name}: {document}"))
}
