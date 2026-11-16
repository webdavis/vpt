use crate::fixture::{Trial, script};
use crate::support::{stderr, stdout};
use serde_json::{Value, json};
use std::path::PathBuf;

const TOKEN: &str = "made-up-self-test-fixture-not-a-credential";

struct Logs {
    _directory: tempfile::TempDir,
    say: PathBuf,
    whisply: PathBuf,
    token: PathBuf,
    joined: PathBuf,
}

fn wave(samples: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(
        u32::try_from(4 + 24 + 12 + 8 + samples.len())
            .unwrap()
            .to_le_bytes(),
    );
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend([1, 0, 1, 0, 0x80, 0x3e, 0, 0, 0, 0x7d, 0, 0, 2, 0, 16, 0]);
    bytes.extend(b"FLLR");
    bytes.extend(4u32.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend(b"data");
    bytes.extend(u32::try_from(samples.len()).unwrap().to_le_bytes());
    bytes.extend(samples);
    bytes
}

fn speakers(trial: &Trial, labels: &[&str], token: bool) -> Logs {
    let directory = tempfile::tempdir().unwrap();
    let logs = Logs {
        say: directory.path().join("say.log"),
        whisply: directory.path().join("whisply.args"),
        token: directory.path().join("whisply.token"),
        joined: directory.path().join("joined.wav"),
        _directory: directory,
    };
    let part = trial.path().join("line.wav");
    std::fs::write(&part, wave(b"made-up samples")).unwrap();
    script(
        &trial.sandbox,
        "say",
        &format!(
            "printf '%s\\n' \"$@\" >> {:?}\nwhile [ $# -gt 0 ]; do [ \"$1\" = -o ] && out=$2; shift; done\n/bin/cp {part:?} \"$out\"",
            logs.say
        ),
    );
    let clip = trial.path().join("clip-fixture.m4a");
    script(
        &trial.sandbox,
        "afconvert",
        &format!(
            "[ \"$1 $2 $3 $4\" = '-f m4af -d aac' ] || exit 64\n/bin/cp \"$5\" {:?}\n/bin/cp {clip:?} \"$6\"",
            logs.joined
        ),
    );
    let words: Vec<Value> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            json!({"word": "kettle", "start": index as f64, "end": index as f64 + 0.5,
                "speaker": label})
        })
        .collect();
    let document = trial.path().join("whisply.json");
    let output = json!({"device": "mlx", "model": "large-v3-turbo", "transcription": {"en": {
        "text": "kettle", "chunks": [{"timestamp": [0.0, labels.len() as f64], "text": "kettle",
        "words": words}]}}});
    std::fs::write(&document, output.to_string()).unwrap();
    let whisply = script(
        &trial.sandbox,
        "fake-whisply",
        &format!(
            "printf '%s\\n' \"$@\" > {:?}\nprintf '%s' \"${{HF_TOKEN-unset}}\" > {:?}\n[ -f {:?} ] && exit 3\n/bin/mkdir -p \"$5/audio\"\n/bin/cat {document:?} > \"$5/audio/audio.json\"",
            logs.whisply,
            logs.token,
            trial.path().join("whisply.fail"),
        ),
    );
    let token = if token {
        format!("hf_token = {TOKEN:?}\n")
    } else {
        String::new()
    };
    trial.configure(&format!(
        "[engines]\nmain = 'labels'\n[engines.labels]\nkind = 'whisply'\ncommand = [{whisply:?}]\n{token}"
    ));
    logs
}

fn check(document: &Value) -> &Value {
    let checks = document["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1, "{document}");
    assert_eq!(checks[0]["name"], "speakers");
    &checks[0]
}

fn row<'a>(document: &'a Value, name: &str) -> &'a Value {
    check(document)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row {name}: {document}"))
}

#[test]
fn two_speaker_labels_are_ok_and_the_row_says_what_left_the_machine() {
    let trial = Trial::new("test-speakers-two");
    let logs = speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], true);
    let before = crate::observations_fixture::snapshot(&trial.sandbox);
    let output = trial.output(&["test", "speakers", "-o", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!stdout(&output).contains(TOKEN));
    let document: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(row(&document, "clip")["result"], "ok", "{document}");
    let labels = row(&document, "labels");
    assert_eq!(labels["result"], "ok", "{document}");
    let detail = labels["detail"].as_str().unwrap();
    assert!(detail.starts_with("2 speaker labels in "), "{detail}");
    assert!(detail.contains("Hugging Face"), "{detail}");
    assert!(detail.contains("first use"), "{detail}");
    assert_eq!(labels["token_sent"], true, "{document}");
    assert_eq!(std::fs::read_to_string(&logs.token).unwrap(), TOKEN);
    let arguments = std::fs::read_to_string(&logs.whisply).unwrap();
    assert!(arguments.lines().any(|line| line == "-a"), "{arguments}");
    assert_eq!(
        crate::observations_fixture::snapshot(&trial.sandbox),
        before
    );
    assert!(trial.temp_is_empty());
}

#[test]
fn the_clip_joins_two_lines_spoken_by_two_voices() {
    let trial = Trial::new("test-speakers-voices");
    let logs = speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], true);
    trial.json(&["test", "speakers"], 0);
    let said = std::fs::read_to_string(&logs.say).unwrap();
    let voices: Vec<&str> = said
        .lines()
        .zip(said.lines().skip(1))
        .filter(|(flag, _)| *flag == "-v")
        .map(|(_, voice)| voice)
        .collect();
    assert_eq!(voices.len(), 2, "{said}");
    assert_ne!(voices[0], voices[1]);
    let joined = std::fs::read(&logs.joined).unwrap();
    assert!(joined.starts_with(b"RIFF"));
    assert!(joined.ends_with(b"made-up samplesmade-up samples"));
}

#[test]
fn one_speaker_label_is_an_issue_with_advice() {
    let trial = Trial::new("test-speakers-one");
    speakers(&trial, &["SPEAKER_00", "SPEAKER_00"], true);
    let document = trial.json(&["test", "speakers"], 1);
    let labels = row(&document, "labels");
    assert_eq!(labels["result"], "issue");
    assert!(
        labels["detail"]
            .as_str()
            .unwrap()
            .starts_with("1 speaker label in ")
    );
    assert!(labels["advice"].as_str().is_some());
}

#[test]
fn a_failed_annotation_names_the_pyannote_terms_in_its_advice() {
    let trial = Trial::new("test-speakers-gated");
    speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], true);
    std::fs::write(trial.path().join("whisply.fail"), "").unwrap();
    let document = trial.json(&["test", "speakers"], 1);
    let labels = row(&document, "labels");
    assert_eq!(labels["result"], "issue");
    let advice = labels["advice"].as_str().unwrap();
    assert!(advice.contains("accept the terms"), "{advice}");
    assert!(
        advice.contains("pyannote/speaker-diarization-3.1"),
        "{advice}"
    );
    assert!(advice.contains("pyannote/segmentation-3.0"), "{advice}");
    assert!(trial.temp_is_empty());
}

#[test]
fn a_missing_afconvert_is_a_clip_issue_and_whisply_never_runs() {
    let trial = Trial::new("test-speakers-no-afconvert");
    let logs = speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], true);
    std::fs::remove_file(trial.path().join("bin/afconvert")).unwrap();
    let document = trial.json(&["test", "speakers"], 1);
    let clip = row(&document, "clip");
    assert_eq!(clip["result"], "issue");
    assert!(clip["detail"].as_str().unwrap().contains("`afconvert`"));
    assert!(!logs.whisply.exists());
    assert!(trial.temp_is_empty());
}

#[test]
fn without_a_token_a_named_speakers_test_is_refused_naming_the_key() {
    let trial = Trial::new("test-speakers-no-token");
    let logs = speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], false);
    let output = trial.output(&["test", "speakers"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("`engines.labels.hf_token`"),
        "{}",
        stderr(&output)
    );
    assert!(!logs.say.exists());
}

#[test]
fn a_token_on_an_unselected_whisply_is_refused_naming_the_selection() {
    let trial = Trial::new("test-speakers-unselected");
    speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], true);
    let config = std::fs::read_to_string(trial.sandbox.config_path()).unwrap();
    std::fs::write(
        trial.sandbox.config_path(),
        config.replace("main = 'labels'", "main = 'apple'"),
    )
    .unwrap();
    let output = trial.output(&["test", "speakers"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("`engines.main`"),
        "{}",
        stderr(&output)
    );
}

fn engines_run(trial: &Trial) -> (Value, String) {
    let clip = trial.path().join("clip-fixture.m4a");
    script(
        &trial.sandbox,
        "say",
        &format!("[ \"$1\" = -o ] || exit 64\n/bin/cp {clip:?} \"$2\""),
    );
    let human = trial.output(&["test", "engines"]);
    let json = trial.output(&["test", "engines", "-o", "json"]);
    for output in [&human, &json] {
        assert!(!stdout(output).contains(TOKEN));
        assert!(!stderr(output).contains(TOKEN));
    }
    let document: Value = serde_json::from_str(&stdout(&json)).unwrap();
    (document, stdout(&human))
}

#[test]
fn an_engines_row_says_when_whisply_got_the_token() {
    let trial = Trial::new("test-engines-token");
    speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], true);
    let (document, human) = engines_run(&trial);
    let labels = crate::fixture::row(&document, "labels");
    let detail = labels["detail"].as_str().unwrap();
    assert!(
        detail.contains("; sent the Hugging Face token to the engine"),
        "{detail}"
    );
    assert_eq!(labels["token_sent"], true, "{document}");
    assert!(
        human.contains("sent the Hugging Face token to the engine"),
        "{human}"
    );
}

#[test]
fn an_engines_row_without_a_token_says_nothing_of_one() {
    let trial = Trial::new("test-engines-no-token");
    speakers(&trial, &["SPEAKER_00", "SPEAKER_01"], false);
    let (document, human) = engines_run(&trial);
    let labels = crate::fixture::row(&document, "labels");
    assert!(labels.get("token_sent").is_none(), "{document}");
    assert!(!human.contains("Hugging Face"), "{human}");
}
