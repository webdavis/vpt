use crate::{
    fixture,
    support::{self, Sandbox, stderr},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::{FlagLedger, RecordingLedger};

pub(super) struct Fixture {
    pub sandbox: Sandbox,
    pub parent: PathBuf,
    id: String,
    flag: String,
}
impl Fixture {
    pub fn new() -> Self {
        let sandbox = Sandbox::new("config-refusal");
        let engine = sandbox.path().join("bin/engine");
        let calls = sandbox.path().join("engine-calls");
        let document = sandbox.path().join("engine.json");
        fs::write(
            &engine,
            "#!/bin/sh\nprintf 'called\\n' >> \"$1\"\n/bin/cat \"$2\"\n",
        )
        .unwrap();
        fs::set_permissions(&engine, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(&document, serde_json::json!({
            "schema":"vpt.engine/1", "engine":{"name":"custom","tool":"fixture","version":"1","model":"test","family":"independent"},
            "language":"en-US", "local":true, "segments":[{"start":0.0,"end":1.0,"text":"Private words", "words":[{"text":"Private","start":0.0,"end":0.5},{"text":"words","start":0.5,"end":1.0}]}]
        }).to_string()).unwrap();
        fixture::write_config(
            &sandbox,
            &format!(
                "[source]\nrecordings_dir = {:?}\nquiet_period_secs = 1\n[engines]\nmain = 'custom'\n[engines.custom]\nkind = 'command'\nfamily = 'independent'\nlocal = true\ncommand = [{engine:?}, {calls:?}, {document:?}]\n",
                sandbox.path().join("voice-memos/Recordings")
            ),
        );
        let audio = sandbox.path().join("voice-memos/Recordings/one.m4a");
        fs::write(
            &audio,
            vpt_domain::fixtures::m4a(1_787_604_456, 12, b"private audio"),
        )
        .unwrap();
        fs::File::open(audio)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(1_787_604_456))
            .unwrap();
        let output = support::run_with_budget(
            sandbox.vpt().args(["run", "--json"]),
            Duration::from_secs(8),
        );
        assert!(output.status.success(), "{}", stderr(&output));
        let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap();
        let id = ledger.recordings().unwrap().remove(0).id;
        let flag = ledger
            .transcript_flags(&id)
            .unwrap()
            .remove(0)
            .identifier
            .hex();
        let parent = sandbox.path().join("external");
        fs::create_dir(&parent).unwrap();
        let alert = sandbox.path().join("bin/alert");
        fs::write(
            &alert,
            "#!/bin/sh\n/bin/cat >> \"$1\"\nprintf '\\n' >> \"$1\"\n",
        )
        .unwrap();
        fs::set_permissions(&alert, fs::Permissions::from_mode(0o700)).unwrap();
        let config = fs::read_to_string(sandbox.config_path()).unwrap();
        fs::write(sandbox.config_path(), format!("{config}\n[stores]\ndrafts = {:?}\n[alert]\ncommand = [{alert:?}, {:?}]\n[report]\ncommand = ['/usr/bin/touch', {:?}]\n[analysis]\ncommand = ['/usr/bin/false']\n", parent.join("drafts"), sandbox.path().join("alerts.jsonl"), sandbox.path().join("report-called"))).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
        Self {
            sandbox,
            parent,
            id: id.to_string(),
            flag,
        }
    }
    pub fn ledger(&self) -> SqliteLedger {
        SqliteLedger::open_read_only(&self.sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap()
    }
    pub fn args<'a>(&'a self, verb: &'a str) -> Vec<&'a str> {
        match verb {
            "run" => vec![verb],
            "note" => vec![verb, "write", &self.id],
            "confirm" => vec![verb, &self.id, "--tag", "work"],
            "review" => vec![verb, &self.id, "--resolve", &self.flag, "--confirm"],
            "brief" => vec![
                verb,
                "--title",
                "Private planning",
                "--at",
                "2026-10-01T10:00:00-06:00",
            ],
            "upcoming" => vec!["brief", "--upcoming"],
            _ => vec![verb, &self.id],
        }
    }
    pub fn enable_upcoming(&self) {
        use vpt_application::ports::Clock;
        let start = vpt_adapters::SystemClock.now().secs + 3600;
        let calendar = self.sandbox.path().join("calendar.json");
        fs::write(
            &calendar,
            support::dam::agenda(vec![support::dam::event(
                "config-refusal",
                "Private event",
                start,
                start + 1800,
            )]),
        )
        .unwrap();
        let script = self.sandbox.path().join("bin/dam");
        fs::write(
            &script,
            format!("#!/bin/sh\n{}", support::dam::script(&calendar)),
        )
        .unwrap();
        fs::set_permissions(script, fs::Permissions::from_mode(0o700)).unwrap();
        let config = fs::read_to_string(self.sandbox.config_path()).unwrap();
        fs::write(self.sandbox.config_path(), format!("{config}\n[brief.trigger]\nenabled = true\n[context]\ntype = 'dam'\ncalendars = ['calendar/work/']\nprojects = ['work/']\n")).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.parent, fs::Permissions::from_mode(0o700));
    }
}
