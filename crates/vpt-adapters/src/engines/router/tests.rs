use super::*;
use std::fs;
use std::time::Duration;
use vpt_application::engines::{EngineConfig, EngineKind};

#[test]
fn external_engine_kinds_run_without_probing_an_absent_apple_helper() {
    let fixture = crate::test_temp::tempdir().unwrap();
    let audio = fixture.path().join("archive.m4a");
    let outputs = fixture.path().join("outputs");
    fs::write(&audio, b"archived").unwrap();
    fs::create_dir(&outputs).unwrap();
    let adapters = EngineAdapters::new(fixture.path().join("absent-helper"));
    for kind in [
        EngineKind::Command {
            command: vec!["/usr/bin/false".into()],
            family: "custom".into(),
            local: true,
        },
        EngineKind::Whisply {
            command: vec!["/usr/bin/false".into()],
            device: "cpu".into(),
            model: "medium".into(),
            hf_token: None,
        },
    ] {
        let engine = EngineConfig {
            name: "external".into(),
            kind,
        };
        let request = EngineRequest {
            engine: &engine,
            audio: &audio,
            out_dir: &outputs,
            language: "en-US",
            duration: 1.0,
            timeout: Duration::from_secs(3),
        };
        assert_eq!(
            adapters.transcribe(&request).unwrap_err(),
            EngineFailure::Exited(1)
        );
        assert_eq!(fs::read(&audio).unwrap(), b"archived");
    }
}
