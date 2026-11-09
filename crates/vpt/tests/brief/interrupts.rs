use super::*;
use std::fs;
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};

#[test]
fn interrupted_context_stops_collection_without_publishing_manual_or_upcoming_briefs() {
    for signal in ["INT", "TERM", "HUP"] {
        for upcoming in [false, true] {
            let sandbox = fixture::configured("brief-interrupted-context");
            let script = sandbox.path().join("collector.sh");
            let calls = sandbox.path().join("collector-calls");
            fs::write(
                &script,
                format!(
                    "printf x >> \"$1\"\nprintf 'Private collector data' >&2\nkill -{signal} \"$PPID\"\nexec /bin/sleep 10\n"
                ),
            )
            .unwrap();
            fixture::write_config(
                &sandbox,
                &format!(
                    "[brief.trigger]\nenabled=true\n[context]\ntype='dam'\ncalendars=['calendar/work/']\nprojects=['work/']\n[context.dam]\ncommand=['/bin/sh',{:?},{:?}]\n",
                    script, calls
                ),
            );
            let args = if upcoming {
                vec!["brief", "--upcoming", "--json"]
            } else {
                manual(&[])
            };
            let output = invoke(&sandbox, &args);
            assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
            assert!(output.stdout.is_empty());
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["schema"], "vpt.error/1");
            assert_eq!(error["error"]["kind"], "command");
            assert!(
                error["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("interrupted")
            );
            assert!(!stderr(&output).contains("Private"));
            assert_eq!(fs::read(&calls).unwrap(), b"x");
            let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
                .unwrap()
                .unwrap();
            assert!(ledger.occasions().unwrap().is_empty());
            assert!(ledger.pending_publications().unwrap().is_empty());
            let history = ledger.recovery_view().unwrap();
            let run = history
                .records
                .iter()
                .find(|record| record.fact == HistoryFact::RunStarted(RunKind::Brief))
                .unwrap()
                .run;
            assert!(history.records.iter().any(|record| record.run == run
                && record.fact == HistoryFact::RunFinished(RunOutcome::Interrupted)));
            assert!(!sandbox.path().join("home/.vpt/briefs").exists());
        }
    }
}
