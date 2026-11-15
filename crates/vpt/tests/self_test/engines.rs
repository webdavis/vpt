use crate::fixture::{Trial, row, rows, script};
use crate::observations_fixture::snapshot;
use vpt_application::engine_trial::SENTENCE;

fn single(trial: &Trial, local: bool, body: &str) {
    trial.configure(&format!(
        "[engines]\nmain = 'custom'\n{}",
        trial.engine("custom", local, body)
    ));
}

fn hearing(trial: &Trial, local: bool, text: &str) {
    trial.configure(&format!(
        "[engines]\nmain = 'custom'\n{}",
        trial.hearing("custom", local, text)
    ));
}

#[test]
fn a_local_engine_that_hears_the_clip_is_ok_and_nothing_is_kept() {
    let trial = Trial::new("test-engines-heard");
    hearing(&trial, true, SENTENCE);
    let before = snapshot(&trial.sandbox);
    let document = trial.json(&["test", "engines"], 0);
    let custom = row(&document, "custom");
    assert_eq!(custom["result"], "ok", "{document}");
    let detail = custom["detail"].as_str().unwrap();
    assert!(detail.starts_with("heard 21 of 21 words in "), "{detail}");
    assert!(!detail.contains("left the machine"), "{detail}");
    assert_eq!(row(&document, "clip")["result"], "ok");
    assert_eq!(document["needs_attention"], serde_json::json!([]));
    assert_eq!(snapshot(&trial.sandbox), before);
    assert!(trial.temp_is_empty());
}

#[test]
fn a_remote_engine_row_says_the_clip_left_the_machine() {
    let trial = Trial::new("test-engines-remote");
    hearing(&trial, false, SENTENCE);
    let document = trial.json(&["test", "engines"], 0);
    let detail = row(&document, "custom")["detail"].as_str().unwrap();
    assert!(
        detail.ends_with("; the made-up clip left the machine"),
        "{detail}"
    );
}

#[test]
fn an_engine_that_hears_other_words_is_an_issue_with_advice() {
    let trial = Trial::new("test-engines-misheard");
    hearing(&trial, true, "an entirely different remark about weather");
    let document = trial.json(&["test", "engines"], 1);
    let custom = row(&document, "custom");
    assert_eq!(custom["result"], "issue");
    assert!(
        custom["detail"]
            .as_str()
            .unwrap()
            .starts_with("heard 0 of 21 words in ")
    );
    assert!(custom["advice"].as_str().unwrap().contains("language"));
    let attention = document["needs_attention"].as_array().unwrap();
    assert_eq!(attention.len(), 1);
    assert_eq!(attention[0]["check"], "engines");
    assert_eq!(attention[0]["name"], "custom");
}

#[test]
fn a_failing_engine_is_an_issue_with_its_reason_and_leaves_nothing() {
    let trial = Trial::new("test-engines-failed");
    single(&trial, true, "exit 17");
    let before = snapshot(&trial.sandbox);
    let document = trial.json(&["test", "engines"], 1);
    let custom = row(&document, "custom");
    assert_eq!(custom["result"], "issue");
    let detail = custom["detail"].as_str().unwrap();
    assert!(detail.starts_with("failed in "), "{detail}");
    assert!(detail.contains("Exited(17)"), "{detail}");
    assert!(
        custom["advice"]
            .as_str()
            .unwrap()
            .contains("vpt doctor engines")
    );
    assert_eq!(snapshot(&trial.sandbox), before);
    assert!(trial.temp_is_empty());
}

#[test]
fn a_missing_say_is_an_issue_and_no_engine_runs() {
    let trial = Trial::new("test-engines-no-say");
    std::fs::remove_file(trial.path().join("bin/say")).unwrap();
    single(
        &trial,
        true,
        &format!("/usr/bin/touch {:?}", trial.path().join("ran")),
    );
    let document = trial.json(&["test", "engines"], 1);
    let rows = rows(&document);
    assert_eq!(rows.len(), 1, "{document}");
    assert_eq!(rows[0]["name"], "clip");
    assert_eq!(rows[0]["result"], "issue");
    assert!(rows[0]["detail"].as_str().unwrap().contains("`say`"));
    assert!(rows[0]["advice"].as_str().is_some());
    assert!(!trial.path().join("ran").exists());
    assert!(trial.temp_is_empty());
}

#[test]
fn a_failing_say_is_an_issue_and_leaves_nothing() {
    let trial = Trial::new("test-engines-say-fails");
    script(&trial.sandbox, "say", "/usr/bin/touch \"$2\"\nexit 3");
    hearing(&trial, true, SENTENCE);
    let document = trial.json(&["test", "engines"], 1);
    let clip = row(&document, "clip");
    assert_eq!(clip["result"], "issue");
    assert!(
        clip["detail"].as_str().unwrap().contains("exited 3"),
        "{clip}"
    );
    assert!(trial.temp_is_empty());
}

#[test]
fn say_gets_the_made_up_sentence_and_a_private_output_file() {
    let trial = Trial::new("test-engines-say-arguments");
    let log = trial.path().join("say.log");
    let clip = trial.path().join("clip-fixture.m4a");
    script(
        &trial.sandbox,
        "say",
        &format!(
            "/usr/bin/stat -f %Lp \"$(/usr/bin/dirname \"$2\")\" > {log:?}\nprintf '%s\\n' \"$@\" >> {log:?}\n/bin/cp {clip:?} \"$2\""
        ),
    );
    hearing(&trial, true, SENTENCE);
    trial.json(&["test", "engines"], 0);
    let logged = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = logged.lines().collect();
    assert_eq!(lines[0], "700");
    assert_eq!(lines[1], "-o");
    assert!(lines[2].starts_with(trial.temp.path().to_str().unwrap()));
    assert!(lines[2].ends_with(".m4a"));
    assert_eq!(lines[3], SENTENCE);
}

#[test]
fn the_default_apple_engine_runs_through_the_helper() {
    let trial = Trial::new("test-engines-apple");
    let document = trial.json(&["test", "engines"], 0);
    let names: Vec<&str> = rows(&document)
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["clip", "apple"]);
    assert_eq!(row(&document, "apple")["result"], "ok");
}

#[test]
fn every_selected_engine_runs_and_a_language_override_is_named() {
    let trial = Trial::new("test-engines-selection");
    trial.configure(&format!(
        "[engines]\nmain = 'custom'\nchecker = 'second'\n[engines.by_language.fr-FR]\nmain = 'french'\n{}{}{}",
        trial.hearing("custom", true, SENTENCE),
        trial.hearing("second", true, SENTENCE),
        trial.hearing("french", true, SENTENCE),
    ));
    let document = trial.json(&["test"], 0);
    let names: Vec<&str> = rows(&document)
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["clip", "custom", "french", "second"]);
    let french = row(&document, "french")["detail"].as_str().unwrap();
    assert!(
        french.ends_with("; selected for fr-FR, run on English speech"),
        "{french}"
    );
    let custom = row(&document, "custom")["detail"].as_str().unwrap();
    assert!(!custom.contains("selected for"), "{custom}");
}
