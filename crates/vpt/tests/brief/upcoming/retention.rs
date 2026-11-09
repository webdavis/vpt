use super::*;
use vpt_application::ports::{ArtifactKind, RetentionArtifacts, RetentionStatus};

#[test]
fn upcoming_discovery_keeps_expired_brief_absent_even_when_event_details_change() {
    let sandbox = configured();
    let now = SystemClock.now().secs;
    reply(&sandbox, vec![event("one", now + 3600)]);
    json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let occasion = ledger.occasions().unwrap().remove(0);
    let brief = ledger.brief(&occasion.id).unwrap().unwrap();
    fs::File::open(&brief.target)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap()
        + "\n[retention]\nenabled=true\n[retention.hold]\nbriefs='1d'\n";
    fs::write(sandbox.config_path(), config).unwrap();
    sandbox.install_fake_helper();
    json(&invoke(&sandbox, &["retention", "run", "--json"]));
    assert!(!brief.target.exists());
    let emitted = reports(&sandbox);
    for offset in [3600, 7200] {
        reply(&sandbox, vec![event("one", now + offset)]);
        let result = json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]));
        assert_eq!(result["written"], json!([]));
        assert!(!brief.target.exists());
        assert_eq!(ledger.brief(&occasion.id).unwrap().unwrap(), brief);
        assert_eq!(reports(&sandbox), emitted);
        let artifact = ledger
            .retention_artifacts()
            .unwrap()
            .into_iter()
            .find(|a| a.kind == ArtifactKind::Brief)
            .unwrap();
        assert_eq!(artifact.status, RetentionStatus::Expired);
        assert!(artifact.trashed_at.is_some());
    }
    json(&invoke(
        &sandbox,
        &["brief", occasion.id.as_str(), "--json"],
    ));
    assert!(brief.target.exists());
}
