use super::*;

#[test]
fn a_zero_or_pre_1971_creation_time_falls_back_to_the_modification_time() {
    for creation in [-2_082_844_800, 0, 31_535_999] {
        let sandbox = configured();
        sandbox.install_fake_helper();
        let path = sandbox.path().join("voice-memos/Recordings/memo.m4a");
        std::fs::write(&path, m4a(creation, 12, b"private audio")).unwrap();
        std::fs::File::open(&path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_787_604_456))
            .unwrap();
        let output = json(&run(sandbox
            .vpt()
            .args(["ingest", "--json"])
            .env("TZ", "UTC")));
        let ingested = &output["ingested"][0];
        assert_eq!(
            ingested["captured_at_utc"], "2026-08-24T20:47:36Z",
            "creation {creation}"
        );
        assert!(
            ingested["id"]
                .as_str()
                .unwrap()
                .starts_with("2026-08-24T204736-"),
            "{ingested}"
        );
    }
}
