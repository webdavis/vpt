use super::*;
use std::fs;

pub(super) fn configure(sandbox: &Sandbox, mode: &str, same_family: bool, extra: &str) {
    let script = sandbox.path().join("pair.sh");
    fs::write(&script, "#!/bin/sh\nset -eu\nname=$1\nroot=$2\n/bin/mkdir \"$root/running\"\nprintf '%s:start\\n' \"$name\" >> \"$root/pair.calls\"\nprintf '%s\\n' \"$3\" \"$4\" \"$5\" > \"$root/$name.args\"\n/bin/cat \"$3\" > \"$root/$name.input\"\n/bin/cat \"$root/$name.document\"\n/bin/rmdir \"$root/running\"\nprintf '%s:end\\n' \"$name\" >> \"$root/pair.calls\"\nexit \"$(/bin/cat \"$root/$name.exit\")\"\n").unwrap();
    let mut definitions = String::new();
    for (name, local) in [("primary", true), ("secondary", false)] {
        let family = if same_family { "shared" } else { name };
        definitions.push_str(&format!("[engines.{name}]\nkind='command'\nfamily='{family}'\nlocal={local}\ncommand=['/bin/sh', {script:?}, '{name}', {:?}, '{{audio}}', '{{language}}', '{{out_dir}}']\n", sandbox.path()));
        let document = serde_json::json!({"schema":"vpt.engine/1",
            "engine":{"name":name,"tool":"fixture","version":"1","model":"test","family":family},
            "language":"fr-CA","local":local,"segments":[{"start":0.0,"end":1.0,
                "text":format!("Private {name} 42"),"words":[{"text":format!("Private {name} 42"),"start":0.0,"end":1.0,"confidence":0.01}]}]});
        fs::write(
            sandbox.path().join(format!("{name}.document")),
            document.to_string(),
        )
        .unwrap();
        fs::write(sandbox.path().join(format!("{name}.exit")), "0").unwrap();
    }
    fixture::write_config(
        sandbox,
        &format!(
            "[recording]\nlanguage='fr-CA'\n[engines]\nmain='primary'\nchecker='secondary'\nchecker_runs='{mode}'\n{extra}\n{definitions}"
        ),
    );
}

pub(super) fn calls(sandbox: &Sandbox) -> String {
    fs::read_to_string(sandbox.path().join("pair.calls")).unwrap_or_default()
}

pub(super) fn reported_language(sandbox: &Sandbox, name: &str, language: &str) {
    let path = sandbox.path().join(format!("{name}.document"));
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    doc["language"] = language.into();
    fs::write(path, doc.to_string()).unwrap();
}
