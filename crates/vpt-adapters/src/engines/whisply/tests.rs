use super::*;
mod boundaries;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::engines::{EngineConfig, EngineKind};
use vpt_domain::transcript::LanguageSource;

struct Fixture {
    directory: tempfile::TempDir,
    audio: PathBuf,
    output: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = crate::test_temp::tempdir().unwrap();
        let audio = directory.path().join("archive.m4a");
        let output = directory.path().join("outputs");
        fs::write(&audio, b"archived-bytes").unwrap();
        fs::create_dir(&output).unwrap();
        Self {
            directory,
            audio,
            output,
        }
    }

    fn engine(&self, body: &str) -> EngineConfig {
        let executable = self.directory.path().join("fixture engine");
        let script = format!(
            r#"#!/bin/sh
[ "$1" = run ] || exit 7
shift
while [ "$#" -gt 0 ]; do
 case "$1" in
 -f) audio="$2";; -o) output="$2";; -d) device="$2";; -m) model="$2";; -l) language="$2";; -e) export="$2";;
 *) exit 8;;
 esac
 shift 2
done
[ "$device" = cpu ] && [ "$model" = medium ] && [ "$language" = fr ] && [ "$export" = json ] || exit 9
[ "$(cat "$audio")" = archived-bytes ] || exit 10
{body}
"#
        );
        fs::write(&executable, script).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        EngineConfig {
            name: "whisply".into(),
            kind: EngineKind::Whisply {
                command: vec![executable.to_str().unwrap().into()],
                device: "cpu".into(),
                model: "medium".into(),
                hf_token: None,
            },
        }
    }

    fn request<'a>(&'a self, engine: &'a EngineConfig) -> EngineRequest<'a> {
        EngineRequest {
            engine,
            audio: &self.audio,
            out_dir: &self.output,
            language: "fr-CA",
            duration: 1.5,
            timeout: Duration::from_secs(3),
        }
    }
}

const WRITE: &str = r#"
mkdir -p "$output/audio"
printf '%s' '{"device":"cpu","model":"medium","transcription":{"fr":{"text":"Bonjour","chunks":[{"timestamp":[0,1.5],"text":"Bonjour","words":[{"word":"Bonjour","start":0,"end":1.5,"score":0.92}]}]}}}' > "$output/audio/audio.json"
printf '%s' 'progress output is not the document'
"#;

#[test]
fn reads_generated_json_and_preserves_archive_despite_backend_side_effects() {
    let fixture = Fixture::new();
    let engine = fixture.engine(&format!(
        "{WRITE}\nprintf '%s' sidecar > \"${{audio%.m4a}}_converted.wav\"\nrm \"$audio\""
    ));
    let result = WhisplyEngine.transcribe(&fixture.request(&engine)).unwrap();
    assert_eq!(result.transcript.segments[0].words[0].text, "Bonjour");
    assert_eq!(
        result.transcript.segments[0].words[0].confidence,
        Some(0.92)
    );
    assert_eq!(result.transcript.language, "fr-CA");
    assert_eq!(result.transcript.language_source, LanguageSource::Requested);
    assert_eq!(fs::read(&fixture.audio).unwrap(), b"archived-bytes");
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
    assert!(
        !fixture
            .directory
            .path()
            .join("archive_converted.wav")
            .exists()
    );
}

#[test]
fn failure_cleans_temporary_files_and_leaves_original_bytes() {
    let fixture = Fixture::new();
    let engine = fixture.engine("printf '%s' private-canary >&2\nrm \"$audio\"\nexit 17");
    assert_eq!(
        WhisplyEngine
            .transcribe(&fixture.request(&engine))
            .unwrap_err(),
        EngineFailure::Exited(17)
    );
    assert_eq!(fs::read(&fixture.audio).unwrap(), b"archived-bytes");
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn unsupported_language_refuses_before_starting_the_executable_or_writing_files() {
    let fixture = Fixture::new();
    let engine = fixture.engine("exit 8");
    let mut request = fixture.request(&engine);
    request.language = "zxx-US";
    assert_eq!(
        WhisplyEngine.transcribe(&request).unwrap_err(),
        EngineFailure::UnsupportedLanguage
    );
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn redirected_backend_output_is_refused_and_its_destination_stays_untouched() {
    let fixture = Fixture::new();
    let canary = fixture.directory.path().join("canary.json");
    fs::write(&canary, b"private-canary").unwrap();
    let engine = fixture.engine(&format!(
        "mkdir -p \"$output/audio\"\nln -s '{}' \"$output/audio/audio.json\"",
        canary.display()
    ));
    assert!(WhisplyEngine.transcribe(&fixture.request(&engine)).is_err());
    assert_eq!(fs::read(&canary).unwrap(), b"private-canary");
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}

#[test]
fn source_symlinks_are_refused_before_a_working_copy_is_created() {
    let fixture = Fixture::new();
    let link = fixture.directory.path().join("linked.m4a");
    std::os::unix::fs::symlink(&fixture.audio, &link).unwrap();
    let engine = fixture.engine(WRITE);
    let mut request = fixture.request(&engine);
    request.audio = Path::new(&link);
    assert!(WhisplyEngine.transcribe(&request).is_err());
    assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
}
