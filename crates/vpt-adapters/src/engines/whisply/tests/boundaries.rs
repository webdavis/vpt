use super::*;

#[test]
fn malformed_generated_json_fails_privately_and_cleans_the_workspace() {
    let fixture = Fixture::new();
    let engine = fixture.engine(
        "mkdir -p \"$output/audio\"\nprintf '%s' private-canary > \"$output/audio/audio.json\"",
    );
    let error = WhisplyEngine
        .transcribe(&fixture.request(&engine))
        .unwrap_err();
    assert_eq!(error, EngineFailure::InvalidDocument);
    assert!(!format!("{error:?}").contains("private-canary"));
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn an_empty_command_is_refused_before_preparing_audio() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(WRITE);
    if let EngineKind::Whisply { command, .. } = &mut engine.kind {
        command.clear();
    }
    assert_eq!(
        WhisplyEngine
            .transcribe(&fixture.request(&engine))
            .unwrap_err(),
        EngineFailure::InvalidArguments
    );
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn deadline_failure_removes_the_working_copy_and_preserves_the_archive() {
    let fixture = Fixture::new();
    let engine = fixture.engine("sleep 5");
    let mut request = fixture.request(&engine);
    request.timeout = Duration::from_millis(30);
    assert_eq!(
        WhisplyEngine.transcribe(&request).unwrap_err(),
        EngineFailure::DeadlineExceeded
    );
    assert_eq!(fs::read(&fixture.audio).unwrap(), b"archived-bytes");
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn oversized_generated_json_is_refused_and_cleaned() {
    let fixture = Fixture::new();
    let engine = fixture.engine("mkdir -p \"$output/audio\"\n/bin/dd if=/dev/zero of=\"$output/audio/audio.json\" bs=1024 count=16385 2>/dev/null");
    assert_eq!(
        WhisplyEngine
            .transcribe(&fixture.request(&engine))
            .unwrap_err(),
        EngineFailure::OutputLimit
    );
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn linked_generated_json_is_refused_without_changing_foreign_permissions() {
    let fixture = Fixture::new();
    let foreign = fixture.directory.path().join("foreign");
    fs::write(&foreign, b"private-canary").unwrap();
    fs::set_permissions(&foreign, fs::Permissions::from_mode(0o644)).unwrap();
    let engine = fixture.engine(&format!(
        "mkdir -p \"$output/audio\"\nln '{}' \"$output/audio/audio.json\"",
        foreign.display()
    ));
    assert_eq!(
        WhisplyEngine
            .transcribe(&fixture.request(&engine))
            .unwrap_err(),
        EngineFailure::Io
    );
    assert_eq!(fs::read(&foreign).unwrap(), b"private-canary");
    assert_eq!(
        fs::metadata(&foreign).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn cleanup_refusal_reports_the_retained_directory_without_deleting_replacement_content() {
    let fixture = Fixture::new();
    let engine = fixture.engine("mv \"$output\" \"$output-saved\"\nmkdir \"$output\"\nprintf '%s' replacement > \"$output/canary\"\nexit 1");
    let error = WhisplyEngine
        .transcribe(&fixture.request(&engine))
        .unwrap_err();
    let EngineFailure::WorkspaceCleanup(path) = error else {
        panic!("wrong failure: {error:?}");
    };
    assert_eq!(fs::read(path.join("canary")).unwrap(), b"replacement");
    assert_eq!(fs::read(&fixture.audio).unwrap(), b"archived-bytes");
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 2);
}

#[test]
fn working_audio_and_its_parent_are_private() {
    let fixture = Fixture::new();
    let engine = fixture.engine(&format!("[ \"$(stat -f %Lp \"$audio\")\" = 600 ] || exit 11\n[ \"$(stat -f %Lp \"$output\")\" = 700 ] || exit 12\n{WRITE}"));
    assert!(WhisplyEngine.transcribe(&fixture.request(&engine)).is_ok());
}
